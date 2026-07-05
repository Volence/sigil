//! Integration tests for comptime sum types + exhaustive `match` (Spec 2,
//! Plan 3 — T6). Each case parses a full `.emp` file (asserting a clean
//! parse), then evaluates a named `const` whose value constructs and/or
//! matches a `comptime enum`, and asserts on the resulting [`Value`] /
//! diagnostics.
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

fn enum_val(ty: &str, variant: &str, payload: Vec<Value>) -> Value {
    Value::Enum { ty_name: ty.to_string(), variant: variant.to_string(), payload }
}

// ---- payload construction ----------------------------------------------

#[test]
fn payload_variant_construction() {
    let src = "module m\ncomptime enum Opt { Some(int), None }\nconst R = Opt.Some(5)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(enum_val("Opt", "Some", vec![int(5)])));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn payload_variant_wrong_arity_errors() {
    let src = "module m\ncomptime enum Opt { Some(int), None }\nconst R = Opt.Some(1, 2)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[enum.payload-arity]")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn bare_payload_variant_reference_errors() {
    // `Opt.Some` with no call — the variant declares a payload, so a bare
    // reference must diagnose rather than silently produce an empty payload.
    let src = "module m\ncomptime enum Opt { Some(int), None }\nconst R = Opt.Some\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("payload") && d.message.contains("Opt.Some(")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn nullary_variant_of_comptime_enum_still_works() {
    // A variant with NO declared payload is unaffected by the T6 changes.
    let src = "module m\ncomptime enum Opt { Some(int), None }\nconst R = Opt.None\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(enum_val("Opt", "None", vec![])));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- exhaustive match, arm selection, payload binding -------------------

#[test]
fn match_selects_arm_and_binds_payload() {
    let src = "module m\ncomptime enum Opt { Some(int), None }\n\
        const R = match Opt.Some(5) { Some(x) => x + 1, None => 0 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(6)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn match_selects_none_arm() {
    let src = "module m\ncomptime enum Opt { Some(int), None }\n\
        const R = match Opt.None { Some(x) => x + 1, None => 0 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(0)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn first_match_wins_with_duplicate_variant_arms() {
    // Two arms both named `Some` — the FIRST one must win.
    let src = "module m\ncomptime enum Opt { Some(int), None }\n\
        const R = match Opt.Some(5) { Some(x) => x + 100, Some(y) => y + 999, None => 0 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(105)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- exhaustiveness ------------------------------------------------------

#[test]
fn non_exhaustive_match_errors_naming_missing_variant() {
    let src = "module m\ncomptime enum Opt { Some(int), None }\n\
        const R = match Opt.Some(5) { Some(x) => x + 1 }\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("[match.non-exhaustive]") && d.message.contains("None")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn non_exhaustive_match_names_multiple_missing_variants() {
    let src = "module m\ncomptime enum Tri { A, B, C }\nconst R = match Tri.A { A => 1 }\n";
    let (_, diags) = eval(src, "R");
    let msg = diags
        .iter()
        .find(|d| d.message.contains("[match.non-exhaustive]"))
        .expect("expected a non-exhaustive diagnostic");
    assert!(msg.message.contains('B'), "diagnostic was {:?}", msg.message);
    assert!(msg.message.contains('C'), "diagnostic was {:?}", msg.message);
}

#[test]
fn wildcard_catch_all_makes_match_exhaustive() {
    let src = "module m\ncomptime enum Opt { Some(int), None }\n\
        const R = match Opt.Some(5) { Some(x) => x + 1, _ => 0 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(6)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn binding_catch_all_makes_match_exhaustive() {
    let src = "module m\ncomptime enum Opt { Some(int), None }\n\
        const R = match Opt.None { Some(x) => x + 1, other => 42 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(42)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- wildcard / whole-value binding patterns -----------------------------

#[test]
fn wildcard_pattern_ignores_value() {
    let src = "module m\ncomptime enum Dir { Up, Down }\nconst R = match Dir.Up { _ => 1 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(1)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn whole_value_binding_pattern_captures_scrutinee() {
    // `x` (bare lowercase) binds the WHOLE scrutinee value — re-matching it in
    // a nested `match` proves the binding is a real, usable enum value.
    let src = "module m\ncomptime enum Opt { Some(int), None }\n\
        const R = match Opt.Some(7) { x => match x { Some(n) => n, None => 0 } }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(7)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- return inside an arm body -------------------------------------------

#[test]
fn return_inside_arm_body_exits_enclosing_fn() {
    let src = "module m\n\
        comptime enum Opt { Some(int), None }\n\
        comptime fn f(x: Opt) -> int {\n\
        \x20   match x {\n\
        \x20       Some(v) => if v > 0 { return v } else { 0 },\n\
        \x20       None => 0,\n\
        \x20   }\n\
        }\n\
        const R = f(Opt.Some(9))\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(9)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn return_inside_arm_body_does_not_leak_when_branch_not_taken() {
    // `v > 0` is false, so the `return` never fires; the fn falls through to
    // its own trailing `match` value (0), and R must NOT be poisoned by a
    // leaked pending return.
    let src = "module m\n\
        comptime enum Opt { Some(int), None }\n\
        comptime fn f(x: Opt) -> int {\n\
        \x20   match x {\n\
        \x20       Some(v) => if v > 0 { return v } else { 0 },\n\
        \x20       None => 0,\n\
        \x20   }\n\
        }\n\
        const R = f(Opt.Some(-3))\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(0)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- nested payload / nested variant pattern -----------------------------

#[test]
fn nested_variant_pattern_binds_inner_payload() {
    let src = "module m\n\
        comptime enum Opt { Some(int), None }\n\
        comptime enum Res { Ok(Opt), Err(string) }\n\
        const R = match Res.Ok(Opt.Some(5)) {\n\
        \x20   Ok(Some(x)) => x,\n\
        \x20   Ok(None) => 0,\n\
        \x20   Err(e) => -1,\n\
        }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(5)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn nested_variant_pattern_mismatch_falls_through_to_next_arm() {
    // `Ok(Some(x))` does not match `Res.Ok(Opt.None)` (inner variant differs)
    // — this is an ordinary non-match, so the SECOND `Ok(None)` arm must win,
    // not an error.
    let src = "module m\n\
        comptime enum Opt { Some(int), None }\n\
        comptime enum Res { Ok(Opt), Err(string) }\n\
        const R = match Res.Ok(Opt.None) {\n\
        \x20   Ok(Some(x)) => x,\n\
        \x20   Ok(None) => 77,\n\
        \x20   Err(e) => -1,\n\
        }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(77)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- Result-shaped enum, matched both ways -------------------------------

#[test]
fn result_shaped_enum_matches_ok_arm() {
    let src = "module m\ncomptime enum Res { Ok(int), Err(string) }\n\
        const R = match Res.Ok(5) { Ok(v) => v, Err(e) => 0 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(5)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn result_shaped_enum_matches_err_arm() {
    let src = "module m\ncomptime enum Res { Ok(int), Err(string) }\n\
        const R = match Res.Err(\"bad\") { Ok(v) => v, Err(e) => -1 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(-1)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn err_payload_string_is_bound_correctly() {
    let src = "module m\ncomptime enum Res { Ok(int), Err(string) }\n\
        const R = match Res.Err(\"bad\") { Ok(v) => \"ok\", Err(e) => e }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (v, diags) = eval_const(&file, "R");
    assert_eq!(v, Some(Value::Str("bad".to_string())));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- match on a non-enum scrutinee ---------------------------------------

#[test]
fn match_on_non_enum_scrutinee_errors() {
    let src = "module m\nconst R = match 5 { _ => 1 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("non-enum")),
        "diagnostics were {diags:?}"
    );
}

// ---- nested payload arity mismatch ---------------------------------------

#[test]
fn nested_pattern_arity_mismatch_errors() {
    let src = "module m\n\
        comptime enum Opt { Some(int), None }\n\
        comptime enum Res { Ok(Opt), Err(string) }\n\
        const R = match Res.Ok(Opt.Some(5)) {\n\
        \x20   Ok(Some(x, y)) => x,\n\
        \x20   Ok(None) => 0,\n\
        \x20   Err(e) => -1,\n\
        }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[match.pattern-arity]")),
        "diagnostics were {diags:?}"
    );
}

// ---- T6 review regressions -----------------------------------------------

#[test]
fn nested_arity_error_does_not_cascade_unknown_name() {
    // CRITICAL (D-P2.9): `Res.Ok(Opt.Some(1, 2))` fails to construct (inner
    // arity error, poisoning the payload). Matching it with `Ok(Some(x)) => x`
    // must NOT then fire a spurious `unknown name x` off the arm body — the
    // one real diagnostic is the inner arity error. Exactly one diagnostic.
    let src = "module m\n\
        comptime enum Opt { Some(int), None }\n\
        comptime enum Res { Ok(Opt), Err(string) }\n\
        const R = match Res.Ok(Opt.Some(1, 2)) {\n\
        \x20   Ok(Some(x)) => x,\n\
        \x20   Ok(None) => 0,\n\
        \x20   Err(e) => -1,\n\
        }\n";
    let (_, diags) = eval(src, "R");
    assert!(
        !diags.iter().any(|d| d.message.contains("unknown name")),
        "spurious cascade diagnostic present: {diags:?}"
    );
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("[enum.payload-arity]"),
        "the one diagnostic should be the inner arity error: {diags:?}"
    );
}

#[test]
fn non_exhaustive_hit_reports_exactly_once() {
    // IMPORTANT 1: the common non-exhaustive path — a real non-exhaustive
    // match evaluated against the uncovered variant — must NOT double-report
    // (`[match.non-exhaustive]` AND `no arm matched`). Exactly one diagnostic.
    let src = "module m\ncomptime enum Opt { Some(int), None }\n\
        const R = match Opt.None { Some(x) => x + 1 }\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected exactly one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("[match.non-exhaustive]"),
        "the one diagnostic should be the non-exhaustive error: {diags:?}"
    );
    assert!(
        !diags.iter().any(|d| d.message.contains("no arm matched")),
        "runtime fallback double-reported: {diags:?}"
    );
}

#[test]
fn named_payload_arg_poisons_result() {
    // IMPORTANT 2: a named argument to a payload constructor is a diagnostic,
    // and the result MUST be Poison (not a normal Enum) so a caller checking
    // `== Poison` sees the bad construction.
    let src = "module m\ncomptime enum Opt { Some(int), None }\nconst R = Opt.Some(x: 5)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("positional")),
        "diagnostics were {diags:?}"
    );
}

#[test]
fn typod_variant_pattern_is_caught_even_with_catch_all() {
    // IMPORTANT 3: a pattern naming a nonexistent variant must be diagnosed
    // even when a catch-all `_` would otherwise swallow it — otherwise a typo
    // silently defeats exhaustive match's totality.
    let src = "module m\ncomptime enum Opt { Some(int), None }\n\
        const R = match Opt.Some(5) { Sme(x) => 1, _ => 0 }\n";
    let (_, diags) = eval(src, "R");
    assert!(
        diags.iter().any(|d| d.message.contains("no variant `Sme`")),
        "typo'd variant not caught: {diags:?}"
    );
}

#[test]
fn return_inside_payload_constructor_arg_propagates() {
    // MISSING TEST: a `return` inside a payload-constructor argument belongs
    // to the enclosing fn, not the construction. `return Opt.None` must exit
    // `f`, so R = Opt.None, with no spurious diagnostic.
    let src = "module m\n\
        comptime enum Opt { Some(int), None }\n\
        comptime fn f(c: int) -> Opt {\n\
        \x20   let r = Opt.Some(if c > 0 { return Opt.None } else { 5 })\n\
        \x20   return r\n\
        }\n\
        const R = f(1)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(enum_val("Opt", "None", vec![])));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn payload_constructor_arg_return_not_taken_still_constructs() {
    // Companion to the above: when the `return` branch is NOT taken, the
    // construction completes normally (no leaked pending return).
    let src = "module m\n\
        comptime enum Opt { Some(int), None }\n\
        comptime fn f(c: int) -> Opt {\n\
        \x20   let r = Opt.Some(if c > 0 { return Opt.None } else { 5 })\n\
        \x20   return r\n\
        }\n\
        const R = f(-1)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(enum_val("Opt", "Some", vec![int(5)])));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}
