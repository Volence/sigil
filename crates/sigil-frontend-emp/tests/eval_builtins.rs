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
