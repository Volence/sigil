//! Integration tests for §6.8 comptime builtins (`len`/`map`/`filter`/`fold`
//! on arrays & ranges; `len`/`find`/`slice`/`val` on strings), lambda
//! evaluation, first-class `fn` references, and struct field access
//! (Spec 2, Plan 2 — T6b). Each case parses a full `.emp` file (asserting a
//! clean parse), evaluates a named `const`, and asserts on the resulting
//! [`Value`] / diagnostics.
//!
//! Parse reminder: postfix `.method` only parses on a PATH receiver, so a
//! builtin on a literal is written via a named binding (`XS.len`), a call form
//! (`s.val()`), or the free/pipe form (`map(xs, f)` / `xs |> map(f)`) — never
//! `.method` directly on a literal like `[1,2,3].len`.
use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;

/// Parse `src` (asserting a clean parse) and evaluate the const named `name`.
fn eval(src: &str, name: &str) -> (Option<Value>, Vec<sigil_span::Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_const(&file, name)
}

/// Evaluate `name` and assert it succeeds with no diagnostics, returning value.
fn ok(src: &str, name: &str) -> Value {
    let (v, diags) = eval(src, name);
    assert!(diags.is_empty(), "unexpected diagnostics for `{name}`: {diags:?}");
    v.expect("value")
}

/// Evaluate `name` and assert it produced at least one diagnostic and Poison.
fn err(src: &str, name: &str) {
    let (v, diags) = eval(src, name);
    assert!(!diags.is_empty(), "expected a diagnostic for `{name}`, got none");
    assert_eq!(v, Some(Value::Poison), "expected Poison for `{name}`");
}

/// Like [`err`], but also pins the wording: some diagnostic must contain `msg`.
fn err_msg(src: &str, name: &str, msg: &str) {
    let (v, diags) = eval(src, name);
    assert_eq!(v, Some(Value::Poison), "expected Poison for `{name}`");
    assert!(
        diags.iter().any(|d| d.message.contains(msg)),
        "expected a diagnostic containing {msg:?} for `{name}`, got {diags:?}"
    );
}

fn int(n: i128) -> Value {
    Value::Int(n)
}

fn arr(ns: &[i128]) -> Value {
    Value::Array(ns.iter().copied().map(Value::Int).collect())
}

fn s(v: &str) -> Value {
    Value::Str(v.to_string())
}

// ---- array / range: len -------------------------------------------------

#[test]
fn array_len_via_bare_path() {
    // `.len` is a bare 2-segment path (no call), resolved in eval_path.
    let src = "module m\nconst XS = [1, 2, 3]\nconst N = XS.len\n";
    assert_eq!(ok(src, "N"), int(3));
}

#[test]
fn range_len_via_bare_path() {
    let src = "module m\nconst RNG = 2..5\nconst L = RNG.len\n";
    assert_eq!(ok(src, "L"), int(3));
}

#[test]
fn empty_range_len_is_zero() {
    let src = "module m\nconst RNG = 5..5\nconst L = RNG.len\n";
    assert_eq!(ok(src, "L"), int(0));
}

// ---- array: map ---------------------------------------------------------

#[test]
fn map_with_lambda() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = XS.map(|x| x * 2)\n";
    assert_eq!(ok(src, "R"), arr(&[2, 4, 6]));
}

#[test]
fn map_with_fn_ref() {
    let src = "module m\ncomptime fn dbl(x: int) -> int { return x * 2 }\nconst XS = [1, 2, 3]\nconst R = XS.map(dbl)\n";
    assert_eq!(ok(src, "R"), arr(&[2, 4, 6]));
}

// ---- array: filter ------------------------------------------------------

#[test]
fn filter_keeps_matching() {
    let src = "module m\nconst XS = [1, 2, 3, 4]\nconst R = XS.filter(|x| x % 2 == 0)\n";
    assert_eq!(ok(src, "R"), arr(&[2, 4]));
}

#[test]
fn filter_non_bool_predicate_errors() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = XS.filter(|x| x + 1)\n";
    err(src, "R");
}

// ---- array: fold --------------------------------------------------------

#[test]
fn fold_sums_with_lambda() {
    let src = "module m\nconst XS = [1, 2, 3, 4]\nconst R = XS.fold(0, |acc, x| acc + x)\n";
    assert_eq!(ok(src, "R"), int(10));
}

#[test]
fn fold_with_fn_ref_combiner() {
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst XS = [1, 2, 3, 4]\nconst R = XS.fold(0, add)\n";
    assert_eq!(ok(src, "R"), int(10));
}

// ---- pipe forms ---------------------------------------------------------

#[test]
fn pipe_map() {
    let src = "module m\nconst R = [1, 2, 3] |> map(|x| x + 1)\n";
    assert_eq!(ok(src, "R"), arr(&[2, 3, 4]));
}

#[test]
fn pipe_chained_map_then_fold() {
    let src = "module m\nconst R = [1, 2, 3] |> map(|x| x + 1) |> fold(0, |a, b| a + b)\n";
    assert_eq!(ok(src, "R"), int(9));
}

#[test]
fn free_form_map() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = map(XS, |x| x + 1)\n";
    assert_eq!(ok(src, "R"), arr(&[2, 3, 4]));
}

// ---- ranges as sequences ------------------------------------------------

#[test]
fn range_map_squares() {
    // `(0..4).map(..)` does not parse (paren receiver is not a path), so bind
    // the range first, then method-call it.
    let src = "module m\nconst R4 = 0..4\nconst M = R4.map(|i| i * i)\n";
    assert_eq!(ok(src, "M"), arr(&[0, 1, 4, 9]));
}

#[test]
fn range_pipe_map() {
    let src = "module m\nconst M = (0..4) |> map(|i| i * i)\n";
    assert_eq!(ok(src, "M"), arr(&[0, 1, 4, 9]));
}

// ---- string: len --------------------------------------------------------

#[test]
fn string_len_via_bare_path() {
    let src = "module m\nconst H = \"hello\"\nconst N = H.len\n";
    assert_eq!(ok(src, "N"), int(5));
}

// ---- string: find -------------------------------------------------------

#[test]
fn string_find_substring() {
    let src = "module m\nconst H = \"hello\"\nconst I = H.find(\"ll\")\n";
    assert_eq!(ok(src, "I"), int(2));
}

#[test]
fn string_find_absent_is_minus_one() {
    let src = "module m\nconst H = \"hello\"\nconst I = H.find(\"z\")\n";
    assert_eq!(ok(src, "I"), int(-1));
}

#[test]
fn string_find_first_char_is_zero() {
    let src = "module m\nconst H = \"hello\"\nconst I = H.find(\"h\")\n";
    assert_eq!(ok(src, "I"), int(0));
}

#[test]
fn string_find_no_last_char_bug() {
    // AS `strstr` has a last-character quirk; ours is standard: the FIRST match.
    let a = "module m\nconst H = \"abcabc\"\nconst I = H.find(\"c\")\n";
    assert_eq!(ok(a, "I"), int(2));
    let b = "module m\nconst H = \"abc\"\nconst I = H.find(\"c\")\n";
    assert_eq!(ok(b, "I"), int(2));
}

// ---- string: slice ------------------------------------------------------

#[test]
fn string_slice_inner() {
    let src = "module m\nconst H = \"hello\"\nconst S = H.slice(1, 4)\n";
    assert_eq!(ok(src, "S"), s("ell"));
}

#[test]
fn string_slice_full() {
    let src = "module m\nconst H = \"hello\"\nconst S = H.slice(0, 5)\n";
    assert_eq!(ok(src, "S"), s("hello"));
}

#[test]
fn string_slice_end_out_of_range_errors() {
    let src = "module m\nconst H = \"hi\"\nconst S = H.slice(0, 5)\n";
    err(src, "S");
}

#[test]
fn string_slice_start_after_end_errors() {
    let src = "module m\nconst H = \"hi\"\nconst S = H.slice(2, 1)\n";
    err(src, "S");
}

// ---- string: val --------------------------------------------------------

#[test]
fn string_val_decimal() {
    // `val` takes no args; a bare `"42".val` cannot parse (literal receiver),
    // so `val` is written as the call form `s.val()` on a named binding.
    let src = "module m\nconst S = \"42\"\nconst V = S.val()\n";
    assert_eq!(ok(src, "V"), int(42));
}

#[test]
fn string_val_hex_dollar() {
    let src = "module m\nconst S = \"$ff\"\nconst V = S.val()\n";
    assert_eq!(ok(src, "V"), int(255));
}

#[test]
fn string_val_binary() {
    let src = "module m\nconst S = \"0b101\"\nconst V = S.val()\n";
    assert_eq!(ok(src, "V"), int(5));
}

#[test]
fn string_val_negative() {
    let src = "module m\nconst S = \"-7\"\nconst V = S.val()\n";
    assert_eq!(ok(src, "V"), int(-7));
}

#[test]
fn string_val_bad_errors() {
    let src = "module m\nconst S = \"xyz\"\nconst V = S.val()\n";
    err(src, "V");
}

// ---- struct field access ------------------------------------------------

#[test]
fn struct_field_access() {
    let src = "module m\nconst P = Point{x: 10, y: 20}\nconst X = P.x\n";
    assert_eq!(ok(src, "X"), int(10));
}

#[test]
fn struct_unknown_field_errors() {
    let src = "module m\nconst P = Point{x: 10, y: 20}\nconst Z = P.z\n";
    err(src, "Z");
}

// ---- first-class fn references ------------------------------------------

#[test]
fn bare_fn_name_is_fn_ref() {
    let src = "module m\ncomptime fn dbl(x: int) -> int { return x * 2 }\nconst F = dbl\n";
    assert_eq!(ok(src, "F"), Value::FnRef("dbl".to_string()));
}

// ---- builtins are not user-shadowable (D-P2.10) -------------------------

#[test]
fn builtin_beats_user_fn_of_same_name() {
    // A user `comptime fn len` must NOT intercept the `len` builtin.
    let src = "module m\ncomptime fn len(x: int) -> int { return 999 }\nconst XS = [1, 2, 3]\nconst N = len(XS)\n";
    assert_eq!(ok(src, "N"), int(3));
}

// ---- misc: type mismatch ------------------------------------------------

#[test]
fn map_on_int_errors() {
    let src = "module m\nconst N = 5\nconst R = map(N, |x| x + 1)\n";
    err(src, "R");
}

#[test]
fn find_on_array_errors() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = find(XS, 2)\n";
    err(src, "R");
}

// ---- pinned diagnostic wording (D-P2 error paths) -----------------------

#[test]
fn map_on_int_message_names_type() {
    let src = "module m\nconst N = 5\nconst R = map(N, |x| x + 1)\n";
    err_msg(src, "R", "not defined on int");
}

#[test]
fn filter_non_bool_message() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = XS.filter(|x| x + 1)\n";
    err_msg(src, "R", "must return bool");
}

#[test]
fn slice_out_of_range_message() {
    let src = "module m\nconst H = \"hi\"\nconst S = H.slice(0, 5)\n";
    err_msg(src, "S", "out of range for string of length 2");
}

#[test]
fn val_bad_message() {
    let src = "module m\nconst S = \"xyz\"\nconst V = S.val()\n";
    err_msg(src, "V", "cannot parse `xyz` as an integer");
}

// ---- builtin arity errors ----------------------------------------------

#[test]
fn map_with_no_fn_is_arity_error() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = XS.map()\n";
    err_msg(src, "R", "`map` expects 1 argument(s), got 0");
}

#[test]
fn len_with_arg_is_arity_error() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = XS.len(1)\n";
    err_msg(src, "R", "`len` expects 0 argument(s), got 1");
}

#[test]
fn fold_wrong_arg_count_is_arity_error() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = XS.fold(0)\n";
    err_msg(src, "R", "`fold` expects 2 argument(s), got 1");
}

// ---- callable arity / not-callable --------------------------------------

#[test]
fn lambda_arity_mismatch_errors() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = XS.map(|a, b| a + b)\n";
    err_msg(src, "R", "lambda expects 2 argument(s), got 1");
}

#[test]
fn fn_ref_arity_mismatch_errors() {
    // `add` takes two params; `map` applies it with one → arity error.
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst XS = [1, 2, 3]\nconst R = XS.map(add)\n";
    err_msg(src, "R", "function `add` expects 2 argument(s), got 1");
}

#[test]
fn non_callable_map_arg_errors() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst R = XS.map(5)\n";
    err_msg(src, "R", "value of type int is not callable");
}

// ---- empty sequences ----------------------------------------------------

#[test]
fn map_on_empty_array() {
    let src = "module m\nconst E = []\nconst R = E.map(|x| x + 1)\n";
    assert_eq!(ok(src, "R"), Value::Array(vec![]));
}

#[test]
fn fold_on_empty_array_returns_init() {
    let src = "module m\nconst E = []\nconst R = E.fold(7, |a, b| a + b)\n";
    assert_eq!(ok(src, "R"), int(7));
}

// ---- string edge cases --------------------------------------------------

#[test]
fn find_empty_needle_is_zero() {
    let src = "module m\nconst H = \"hello\"\nconst I = H.find(\"\")\n";
    assert_eq!(ok(src, "I"), int(0));
}

#[test]
fn slice_empty_range_is_empty_string() {
    let src = "module m\nconst H = \"hello\"\nconst S = H.slice(2, 2)\n";
    assert_eq!(ok(src, "S"), s(""));
}

#[test]
fn slice_negative_start_errors() {
    let src = "module m\nconst H = \"hello\"\nconst S = H.slice(-1, 2)\n";
    err(src, "S");
}

// ---- val: bare-path form + more spellings + sign rejection --------------

#[test]
fn val_bare_path_form() {
    // `s.val` (no parens) resolves in eval_path, mirroring `s.len`.
    let src = "module m\nconst S = \"42\"\nconst V = S.val\n";
    assert_eq!(ok(src, "V"), int(42));
}

#[test]
fn val_hex_0x_spelling() {
    let src = "module m\nconst S = \"0x1f\"\nconst V = S.val()\n";
    assert_eq!(ok(src, "V"), int(31));
}

#[test]
fn val_binary_0b_spelling() {
    let src = "module m\nconst S = \"0b101\"\nconst V = S.val()\n";
    assert_eq!(ok(src, "V"), int(5));
}

#[test]
fn val_rejects_leading_plus() {
    let src = "module m\nconst S = \"+5\"\nconst V = S.val()\n";
    err(src, "V");
}

#[test]
fn val_rejects_sign_after_dollar() {
    let src = "module m\nconst S = \"$-5\"\nconst V = S.val()\n";
    err(src, "V");
}

#[test]
fn val_rejects_plus_after_dollar() {
    let src = "module m\nconst S = \"$+5\"\nconst V = S.val()\n";
    err(src, "V");
}

// ---- a string builtin on a range/array reports the surface type ---------
// (`bogus` is not a builtin at all, so it is a silent Poison; a *string*
// builtin name like `find` reaches the sequence dispatcher's fall-through and
// exercises the receiver-type message.)

#[test]
fn string_builtin_on_range_reports_range() {
    let src = "module m\nconst R = 0..4\nconst X = R.find(0)\n";
    err_msg(src, "X", "`find` is not defined on range");
}

#[test]
fn string_builtin_on_array_reports_array() {
    let src = "module m\nconst XS = [1, 2, 3]\nconst X = XS.slice(0, 1)\n";
    err_msg(src, "X", "`slice` is not defined on array");
}

// ---- lambda captures its defining environment BY VALUE ------------------

#[test]
fn lambda_captures_binding_by_value() {
    // `g` closes over `base` when defined (base == 10). A later `base = 99`
    // reassignment must NOT be seen by `g` (capture is by value), so mapping
    // [1, 2] through `g` gives [11, 12], summing to 23 (not 209).
    let src = "module m\n\
comptime fn f() -> int {\n\
    comptime var base = 10\n\
    let g = |x| x + base\n\
    base = 99\n\
    let xs = [1, 2]\n\
    let mapped = xs.map(g)\n\
    return mapped |> fold(0, |a, b| a + b)\n\
}\n\
const R = f()\n";
    assert_eq!(ok(src, "R"), int(23));
}

// ---- whole-branch review: cross-feature seam bugs -----------------------

// (1) A `return` inside a lambda body must yield FROM the lambda, not leak out
// through map/filter/fold to become the enclosing fn's return.

#[test]
fn lambda_return_yields_from_lambda_not_enclosing_fn() {
    // Without the fix this returned Int(99) (the leaked lambda return) with no
    // diagnostics; correct is: the lambda returns 99 for elements > 1, map gives
    // [1, 99, 99], and `go` returns its length, 3.
    let src = "module m\n\
comptime fn go() -> int {\n\
    let xs = [1, 2, 3]\n\
    let ys = xs.map(|x| if x > 1 { return 99 } else { x })\n\
    return ys.len\n\
}\n\
const R = go()\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(3)));
    assert!(diags.is_empty(), "expected no diagnostics, got {diags:?}");
}

#[test]
fn lambda_return_value_becomes_mapped_element() {
    // Proves [1, 99, 99] concretely: summing the mapped array gives 199.
    let src = "module m\n\
comptime fn go() -> int {\n\
    let xs = [1, 2, 3]\n\
    return xs.map(|x| if x > 1 { return 99 } else { x }) |> fold(0, |a, b| a + b)\n\
}\n\
const R = go()\n";
    assert_eq!(ok(src, "R"), int(1 + 99 + 99));
}

// (2) A locally-bound lambda / const-bound fn-ref must be callable by name.

#[test]
fn call_locally_bound_lambda_by_name() {
    let src = "module m\n\
comptime fn go() -> int {\n\
    let f = |x| x + 1\n\
    return f(10)\n\
}\n\
const R = go()\n";
    assert_eq!(ok(src, "R"), int(11));
}

#[test]
fn call_const_bound_fn_ref_by_name() {
    let src = "module m\ncomptime fn dbl(x: int) -> int { return x * 2 }\nconst G = dbl\nconst R = G(5)\n";
    assert_eq!(ok(src, "R"), int(10));
}

#[test]
fn calling_non_callable_value_errors() {
    let src = "module m\n\
comptime fn go() -> int {\n\
    let n = 3\n\
    return n(1)\n\
}\n\
const R = go()\n";
    err_msg(src, "R", "value of type int is not callable");
}

#[test]
fn local_binding_does_not_break_top_level_fn_call() {
    // A same-shaped call to a real top-level fn (no shadowing binding) still
    // dispatches to `self.fns` — the call-by-name path must not regress this.
    let src = "module m\ncomptime fn add(a: int, b: int) -> int { return a + b }\nconst R = add(2, 3)\n";
    assert_eq!(ok(src, "R"), int(5));
}

// (3) Range builtins must not materialize eagerly: `len` is O(1), and
// map/filter/fold charge a step per element so a huge range trips the budget.

#[test]
fn range_len_call_form_is_o1() {
    // `(2..5).len()` → 3 via the call form (eval_builtin), computed without
    // materializing the range.
    let src = "module m\nconst R = 2..5\nconst L = R.len()\n";
    assert_eq!(ok(src, "L"), int(3));
}

#[test]
fn huge_range_map_hits_budget_not_oom() {
    // A billion-element range would be ~16 GB if materialized; the per-element
    // step charge must trip the comptime step budget instead. Aborts after
    // ~budget iterations (sub-second .. a couple seconds in debug), NOT an OOM.
    let src = "module m\nconst BIG = 0..1000000000\nconst R = BIG.map(|i| i)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("step budget exceeded")),
        "expected a step-budget diagnostic, got {diags:?}"
    );
}
