//! Integration tests for the `import(path)` comptime builtin (Spec 2, Plan 5 —
//! Task 2): reads a JSON or TOML file at comptime, within the capability
//! sandbox rooted at a fixed `include_root` (the same guard `embed` uses —
//! `tests/sandbox_embed.rs`), and maps it into generic comptime `Value`s —
//! `Value::Struct { ty_name: "<import>", .. }` for an object/table,
//! `Value::Array` for an array, `Value::Int`/`Value::Float` for a number
//! (integral vs fractional), `Value::Str`/`Value::Bool` for a string/bool, and
//! `Value::Unit` for JSON `null`.
//!
//! Also exercises the struct SHAPE check `lower_struct` now performs (Spec 2,
//! Plan 5 — D-P5.4): an imported object whose keys don't exactly match a
//! typed struct's declared fields is a diagnostic, not a silent mis-size.
use sigil_frontend_emp::eval::eval_const_with_root;
use sigil_frontend_emp::layout::eval_data_with_root;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Cell, DataBuf, Value};
use sigil_span::Diagnostic;
use std::path::{Path, PathBuf};

/// The fixture directory `import` resolves paths against for every test here:
/// `tests/vectors/`, containing `import_fixture.{json,toml}`, `point.json`,
/// `point_missing.json`, `point_extra.json`, `import_bad_format.txt`, and
/// `import_malformed.json`.
fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors")
}

/// Parse `src` (asserting a clean parse) and evaluate the const named `name`,
/// resolving any `import(...)` sandbox path against [`vectors_dir`]. The
/// least-invasive seam for observing an imported `Value` directly (no `data`
/// item / byte layout needed): mirrors `eval_data_with_root`, added alongside
/// `eval_import` as `eval_const_with_root`.
fn const_value(src: &str, name: &str) -> (Option<Value>, Vec<Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_const_with_root(&file, name, Some(&vectors_dir()))
}

/// Parse `src` (asserting a clean parse) and lower the data item named `name`,
/// resolving any `import(...)` sandbox path against [`vectors_dir`].
fn data(src: &str, name: &str) -> (Option<DataBuf>, Vec<Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (buf, _asserts, ds) = eval_data_with_root(&file, name, None, Some(&vectors_dir()));
    (buf, ds)
}

/// Look up field `field` of a struct `Value`, panicking with a useful message
/// if `v` isn't a struct or has no such field.
fn field<'a>(v: &'a Value, field: &str) -> &'a Value {
    match v {
        Value::Struct { fields, .. } => fields
            .iter()
            .find(|(n, _)| n == field)
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("struct has no field `{field}`: {v:?}")),
        other => panic!("expected a struct value, got {other:?}"),
    }
}

/// Serialize a checked [`DataBuf`]'s cells to big-endian bytes — the M68000
/// byte order confirmed by `tests/lower_data.rs`'s `multibyte_scalar_is_big_endian`
/// (a width>1 `Cell::Scalar` serializes big-endian). `DataBuf` itself commits
/// no endianness (that's the Plan-4 `lower_module`/`sigil_link` seam, which does
/// not yet thread a sandbox `include_root` — see `eval_data_with_root`'s doc
/// comment); this local helper lets the typed-import test assert byte-exactness
/// without needing that wiring.
fn cells_to_be_bytes(buf: &DataBuf) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.size);
    for cell in &buf.cells {
        match cell {
            Cell::Scalar { value, width, .. } => {
                let be = value.to_be_bytes();
                let start = be.len() - *width as usize;
                out.extend_from_slice(&be[start..]);
            }
            Cell::Bytes(b) => out.extend_from_slice(b),
            Cell::SymRef { .. } => panic!("unexpected SymRef cell in a plain scalar struct"),
            Cell::RelOffset { .. } => panic!("unexpected RelOffset cell in a plain scalar struct"),
            Cell::Expr { .. } => panic!("unexpected Expr (link-expr) cell in a plain scalar struct"),
        }
    }
    out
}

// ---- untyped import: JSON / TOML -> generic comptime Values ---------------

#[test]
fn import_json_untyped() {
    let src = "module m\nconst V = import(\"import_fixture.json\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let v = v.expect("const value");
    assert_eq!(*field(&v, "a"), Value::Int(1));
    assert_eq!(*field(&v, "b"), Value::Array(vec![Value::Int(2), Value::Int(3)]));
    assert_eq!(*field(&v, "c"), Value::Str("hi".to_string()));
    assert_eq!(*field(&v, "d"), Value::Float(1.5));
    assert_eq!(*field(&v, "e"), Value::Bool(true));
}

#[test]
fn import_toml_untyped() {
    let src = "module m\nconst V = import(\"import_fixture.toml\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let v = v.expect("const value");
    assert_eq!(*field(&v, "a"), Value::Int(1));
    assert_eq!(*field(&v, "b"), Value::Array(vec![Value::Int(2), Value::Int(3)]));
    assert_eq!(*field(&v, "c"), Value::Str("hi".to_string()));
    assert_eq!(*field(&v, "d"), Value::Float(1.5));
    assert_eq!(*field(&v, "e"), Value::Bool(true));
}

// ---- typed struct import: lowers against the declared struct's layout -----

#[test]
fn import_typed_struct() {
    let src = "module m\nstruct Point { x: u16, y: u16 }\ndata P: Point = import(\"point.json\")\n";
    let (buf, diags) = data(src, "P");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 4);
    assert_eq!(cells_to_be_bytes(&buf), vec![0, 10, 0, 20]);
}

#[test]
fn import_shape_mismatch_missing_field() {
    let src = "module m\nstruct Point { x: u16, y: u16 }\ndata P: Point = import(\"point_missing.json\")\n";
    let (_buf, diags) = data(src, "P");
    assert!(
        diags.iter().any(|d| d.message.contains("[struct.missing-field]")),
        "expected a [struct.missing-field] diagnostic, got {diags:?}"
    );
}

#[test]
fn import_shape_mismatch_extra_field() {
    let src = "module m\nstruct Point { x: u16, y: u16 }\ndata P: Point = import(\"point_extra.json\")\n";
    let (_buf, diags) = data(src, "P");
    assert!(
        diags.iter().any(|d| d.message.contains("[struct.unknown-field]")),
        "expected a [struct.unknown-field] diagnostic, got {diags:?}"
    );
}

// ---- error paths: bad extension / malformed content ------------------------

#[test]
fn import_bad_format() {
    let src = "module m\nconst V = import(\"import_bad_format.txt\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(
        diags.iter().any(|d| d.message.contains("[import.format]")),
        "expected an [import.format] diagnostic, got {diags:?}"
    );
    assert_eq!(v, Some(Value::Poison));
}

#[test]
fn import_parse_error() {
    let src = "module m\nconst V = import(\"import_malformed.json\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(
        diags.iter().any(|d| d.message.contains("[import.parse]")),
        "expected an [import.parse] diagnostic, got {diags:?}"
    );
    assert_eq!(v, Some(Value::Poison));
}

#[test]
fn import_path_escape_rejected() {
    // Reuses the shared sandbox guard (`resolve_sandbox_path`) — same
    // `[sandbox.path-escape]` code `embed` reports.
    let src = "module m\nconst V = import(\"../secret.json\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(
        diags.iter().any(|d| d.message.contains("[sandbox.path-escape]")),
        "expected a [sandbox.path-escape] diagnostic, got {diags:?}"
    );
    assert_eq!(v, Some(Value::Poison));
}

#[test]
fn import_missing_file() {
    let src = "module m\nconst V = import(\"does_not_exist.json\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(
        diags.iter().any(|d| d.message.contains("[import.read]")),
        "expected an [import.read] diagnostic, got {diags:?}"
    );
    assert_eq!(v, Some(Value::Poison));
}

// ---- number/null/datetime mapping edge cases (T2 review follow-up) --------

#[test]
fn import_json_null_is_unit() {
    // JSON `null` maps to `Value::Unit` (D-P5.4).
    let src = "module m\nconst V = import(\"import_types.json\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(field(v.as_ref().unwrap(), "n"), &Value::Unit);
}

#[test]
fn import_integral_valued_float_is_float() {
    // `2.0` is stored by serde_json as f64 → `Value::Float`, NOT `Int`
    // (an honest float even though its value is integral).
    let src = "module m\nconst V = import(\"import_types.json\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(field(v.as_ref().unwrap(), "whole"), &Value::Float(2.0));
}

#[test]
fn import_wide_unsigned_int_is_int() {
    // u64::MAX exceeds i64 but fits u64 → `is_u64()` → `Value::Int(i128)`.
    let src = "module m\nconst V = import(\"import_types.json\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        field(v.as_ref().unwrap(), "big"),
        &Value::Int(18_446_744_073_709_551_615_i128)
    );
}

#[test]
fn import_toml_datetime_unsupported() {
    // TOML has a native datetime; there is no comptime equivalent, so it maps
    // to `[import.unsupported]` + `Poison` for that value (D-P5.4).
    let src = "module m\nconst V = import(\"import_datetime.toml\")\n";
    let (v, diags) = const_value(src, "V");
    assert!(
        diags.iter().any(|d| d.message.contains("[import.unsupported]")),
        "expected an [import.unsupported] diagnostic, got {diags:?}"
    );
    // The table is still built; the datetime field alone is Poison.
    assert_eq!(field(v.as_ref().unwrap(), "d"), &Value::Poison);
}
