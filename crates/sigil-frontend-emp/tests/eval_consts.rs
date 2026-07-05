//! Integration tests for const declarations + within-file name resolution
//! (Spec 2, Plan 2 — T3). Each case parses a full `.emp` file (asserting a clean
//! parse), then evaluates a named const via [`eval_const`] and asserts on the
//! resulting [`Value`] and diagnostics.
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

// ---- basic const evaluation -------------------------------------------

#[test]
fn simple_const() {
    let (v, diags) = eval("module m\nconst A = 40 + 2\n", "A");
    assert_eq!(v, Some(int(42)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn const_references_another_const() {
    // Proves lazy resolution + memo: A resolves B on demand.
    let src = "module m\nconst B = 2\nconst A = B + 1\n";
    let (v, diags) = eval(src, "A");
    assert_eq!(v, Some(int(3)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn forward_reference_is_order_independent() {
    // A is defined textually before B but references it — must still resolve.
    let src = "module m\nconst A = B + 1\nconst B = 2\n";
    let (v, diags) = eval(src, "A");
    assert_eq!(v, Some(int(3)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- cycles ------------------------------------------------------------

#[test]
fn direct_cycle_is_poison() {
    let (v, diags) = eval("module m\nconst A = A\n", "A");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("cyclic const"),
        "diagnostic was {:?}",
        diags[0].message
    );
    assert!(diags[0].message.contains("A -> A"), "chain missing: {:?}", diags[0].message);
}

#[test]
fn indirect_cycle_is_poison_and_names_chain() {
    let src = "module m\nconst A = B\nconst B = A\n";
    let (v, diags) = eval(src, "A");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("cyclic const"),
        "diagnostic was {:?}",
        diags[0].message
    );
    // The chain names the full loop back to the repeated const.
    assert!(
        diags[0].message.contains("A -> B -> A"),
        "chain missing: {:?}",
        diags[0].message
    );
}

// ---- unknown names -----------------------------------------------------

#[test]
fn unknown_name_in_const_poisons() {
    let (v, diags) = eval("module m\nconst A = Nope\n", "A");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("unknown name"),
        "diagnostic was {:?}",
        diags[0].message
    );
}

#[test]
fn requesting_nonexistent_const_errors() {
    let (v, diags) = eval("module m\nconst A = 1\n", "MISSING");
    assert!(v.is_none());
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("no const named `MISSING`"),
        "diagnostic was {:?}",
        diags[0].message
    );
}

// ---- struct literal value ----------------------------------------------

#[test]
fn struct_literal_builds_value_in_order() {
    // Point need not be declared — this is value-level only (Plan 3 checks it).
    let src = "module m\nconst P = Point{ x: 1, y: 2 + 3 }\n";
    let (v, diags) = eval(src, "P");
    assert_eq!(
        v,
        Some(Value::Struct {
            ty_name: "Point".into(),
            fields: vec![("x".into(), int(1)), ("y".into(), int(5))],
        })
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- enum variant paths ------------------------------------------------

#[test]
fn enum_variant_path_resolves() {
    let src = "module m\nenum Dir: u8 { Up = 0, Down = 1 }\nconst D = Dir.Up\n";
    let (v, diags) = eval(src, "D");
    assert_eq!(
        v,
        Some(Value::Enum { ty_name: "Dir".into(), variant: "Up".into(), payload: vec![] })
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn enum_unknown_variant_poisons() {
    let src = "module m\nenum Dir: u8 { Up = 0, Down = 1 }\nconst X = Dir.Sideways\n";
    let (v, diags) = eval(src, "X");
    assert_eq!(v, Some(Value::Poison));
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("no variant") || diags[0].message.contains("has no variant"),
        "diagnostic was {:?}",
        diags[0].message
    );
}
