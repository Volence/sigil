//! Integration tests for the `as.*`/`math.*` comptime float namespaces (Spec
//! 2, Plan 5 — Task 4, §6.6). The acceptance gate is
//! [`deform_tables_match_goldens`]: reproducing four real Aeon deform tables
//! byte-for-byte using `as.sin`/`as.int`, matching the proven Core §7.1 /
//! M1.C Spike 0 recipe (Rust std `f64::sin()` + `f64::floor()` bit-match
//! `asl 1.42`'s numeric routines; AS's `int()` is floor, not truncate).
use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::layout::eval_data;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Cell, Value};
use std::path::{Path, PathBuf};

/// The golden-vector directory: `tests/vectors/sine_goldens/`, containing the
/// four committed 256-byte golden tables (copied in by Task 0).
fn goldens_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("vectors").join("sine_goldens")
}

/// Build and lower the deform-table `.emp` program for amplitude `a` and
/// period `p`, returning its single `Cell::Bytes` payload (256 bytes).
/// Mirrors the exact AS operation order: `A * as.sin(2*PI * i / PERIOD)`,
/// left-associative, with `as.int` applying floor (not truncation).
fn deform_bytes(a: i128, p: i128) -> Vec<u8> {
    let src = format!(
        "module m\ndata Deform = bytes(for i in 0..256 {{ as.int({a} * as.sin(6.283185307179586 * i / {p})) }})\n"
    );
    let (file, diags) = parse_str(&src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (buf, diags) = eval_data(&file, "Deform");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.cells.len(), 1, "expected a single Cell::Bytes, got {:?}", buf.cells);
    match &buf.cells[0] {
        Cell::Bytes(b) => {
            assert_eq!(b.len(), 256, "expected 256 bytes, got {}", b.len());
            b.clone()
        }
        other => panic!("expected Cell::Bytes, got {other:?}"),
    }
}

/// THE GATE: four real Aeon deform tables, reproduced byte-for-byte against
/// their committed golden vectors.
#[test]
fn deform_tables_match_goldens() {
    let cases: [(&str, i128, i128); 4] = [
        ("rocking_a20_p64", 20, 64),
        ("ojz_calm_a96_p64", 96, 64),
        ("haze_a16_p64", 16, 64),
        ("shimmer_a8_p32", 8, 32),
    ];
    for (name, a, p) in cases {
        let got = deform_bytes(a, p);
        let golden_path = goldens_dir().join(format!("{name}.bin"));
        let golden = std::fs::read(&golden_path)
            .unwrap_or_else(|e| panic!("failed to read golden {golden_path:?}: {e}"));
        assert_eq!(got, golden, "byte mismatch for {name} (a={a}, p={p})");
    }
}

/// `as.int` is floor toward -infinity (the verified `asl` semantic), NOT
/// truncation toward zero: `floor(-4.2) == -5`, which wraps to the byte
/// `0xFB` in two's complement — truncation would instead give `-4` (`0xFC`).
#[test]
fn as_int_is_floor_not_trunc() {
    let src = "module m\ndata D = bytes([as.int(-4.2 * 1.0)])\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (buf, diags) = eval_data(&file, "D");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.cells, vec![Cell::Bytes(vec![0xFB])]);
}

/// `math.sin` and `as.sin` share the same std-f64 backing today (§6.6): the
/// split is a greppable, eventually-deletable compat surface, not a numeric
/// difference — so both must produce the identical `Value::Float`.
#[test]
fn math_sin_matches_as_sin() {
    let src = "module m\nconst A = as.sin(1.0)\nconst B = math.sin(1.0)\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (a, diags_a) = eval_const(&file, "A");
    let (b, diags_b) = eval_const(&file, "B");
    assert!(diags_a.is_empty(), "unexpected diagnostics: {diags_a:?}");
    assert!(diags_b.is_empty(), "unexpected diagnostics: {diags_b:?}");
    assert_eq!(a, b);
    assert_eq!(a, Some(Value::Float(1.0f64.sin())));
}

/// An unimplemented float function (`as.tan`), and `math.int` (new code
/// should use explicit rounding, not the `as`-only floor compat function),
/// both report `[float-ns.unknown]` and poison — no panic.
#[test]
fn unknown_float_fn_errors() {
    let src = "module m\nconst A = as.tan(1.0)\nconst B = math.int(1.0)\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");

    let (a, diags_a) = eval_const(&file, "A");
    assert_eq!(a, Some(Value::Poison));
    assert!(
        diags_a.iter().any(|d| d.message.contains("[float-ns.unknown]")),
        "diagnostics were {diags_a:?}"
    );

    let (b, diags_b) = eval_const(&file, "B");
    assert_eq!(b, Some(Value::Poison));
    assert!(
        diags_b.iter().any(|d| d.message.contains("[float-ns.unknown]")),
        "diagnostics were {diags_b:?}"
    );
}
