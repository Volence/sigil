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
use sigil_frontend_emp::layout::{eval_data, layout_struct, Ty};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::{Cell, Value};

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

/// The stored int + nominal type of a `const` that must evaluate cleanly to a
/// `Value::Typed` (mirrors the `eval_typed.rs` helper of the same name).
fn typed_stored(src: &str, name: &str) -> (i128, Ty, Vec<sigil_span::Diagnostic>) {
    let (v, diags) = eval(src, name);
    match v {
        Some(Value::Typed { ty, val }) => {
            (val.as_stored_int().expect("typed wraps an int"), *ty, diags)
        }
        other => panic!("expected a Value::Typed, got {other:?} (diags {diags:?})"),
    }
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

// ---- 7. Appendix E — refinement construction (comptime) -----------------
//
// `newtype PaletteLine = u8 where 0..63` mirrors the exhibit's `set_pal`
// bound: construction range-checks at comptime, matching the exhibit's
// compile-time rejection of `set_pal(64)`.

const PALETTE_SRC: &str = "\
module engine.typed

newtype PaletteLine = u8 where 0..63

const OK = PaletteLine(40)
const BAD = PaletteLine(64)
";

#[test]
fn refinement_in_range_construction_is_a_typed_value() {
    let v = ok(PALETTE_SRC, "OK");
    assert_eq!(v.as_stored_int(), Some(40));
    assert!(matches!(v, Value::Typed { .. }), "expected a Value::Typed, got {v:?}");
}

#[test]
fn refinement_out_of_range_construction_is_diagnosed() {
    let (v, diags) = eval(PALETTE_SRC, "BAD");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("64 not in 0..63")),
        "expected the refinement out-of-range diagnostic (mirrors the exhibit's \
         `set_pal(64)` compile error), got {diags:?}"
    );
}

// ---- 8. Appendix E — fixed<> scale (comptime) ----------------------------
//
// `newtype Fix = fixed<16,16>`. Hand-computed stored ints (all 16.16 unless
// noted):
//   A = Fix($10000) = 1.0   -> stored 0x10000 (65536)
//   B = Fix($20000) = 2.0   -> stored 0x20000 (131072)
//   SUM = A + B (same-scale, transparent) = 0x10000 + 0x20000 = 0x30000
//         (196608), still Newtype(Fix).
//   PROD = A * B -> scale DOUBLES to bare fixed<32,32>: stored ints multiply,
//         0x10000 * 0x20000 = 0x2_0000_0000 (8589934592), which read as 32.32
//         is 2.0 — matches 1.0 * 2.0.
//   MISMATCH = PROD + A: a fixed<32,32> plus a newtype-over-fixed<16,16> is a
//         scale mismatch (never a silent shift), naming `rescale<16,16>`.
//   RESCALED = rescale<16,16>(PROD): shifts the stored int right by 16 ->
//         0x2_0000_0000 >> 16 = 0x20000 (131072), a BARE fixed<16,16> (2.0).
const FIX_SRC: &str = "\
module engine.typed

newtype Fix = fixed<16,16>

const A = Fix($10000)
const B = Fix($20000)
const SUM = A + B
const PROD = A * B
const MISMATCH = PROD + A
const RESCALED = rescale<16,16>(PROD)
";

#[test]
fn fixed_scale_same_scale_add_is_transparent() {
    let (n, ty, diags) = typed_stored(FIX_SRC, "SUM");
    assert_eq!(n, 0x30000);
    assert_eq!(ty, Ty::Newtype("Fix".into()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn fixed_scale_multiply_doubles_the_scale() {
    let (n, ty, diags) = typed_stored(FIX_SRC, "PROD");
    assert_eq!(n, 0x10000i128 * 0x20000i128);
    assert_eq!(n, 0x2_0000_0000);
    assert_eq!(ty, Ty::Fixed { i: 32, f: 32 }, "scale doubled to fixed<32,32>");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn fixed_scale_mismatch_names_rescale() {
    let (v, diags) = eval(FIX_SRC, "MISMATCH");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[scale mismatch]")
            && d.message.contains("rescale<16,16>")),
        "expected a scale-mismatch diagnostic naming rescale<16,16>, got {diags:?}"
    );
}

#[test]
fn fixed_scale_rescale_narrows_back_to_fixed_16_16() {
    let (n, ty, diags) = typed_stored(FIX_SRC, "RESCALED");
    assert_eq!(n, 0x2_0000_0000i128 >> 16);
    assert_eq!(n, 0x20000);
    assert_eq!(ty, Ty::Fixed { i: 16, f: 16 });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- 9. Appendix E — newtype distinctness (comptime) ---------------------
//
// `Angle = u8` (plain-prim newtype) and `Pos = fixed<16,16>` (fixed newtype),
// the exact exhibit pairing, exercised end-to-end in one program.
const DISTINCT_SRC: &str = "\
module engine.typed

newtype Angle = u8
newtype Pos = fixed<16,16>

const SUM = Angle(200) + Angle(100)
const MIX = Angle(5) + Pos($10000)
";

#[test]
fn newtype_distinctness_same_type_wraps_at_u8_width() {
    // 200 + 100 = 300 mod 256 = 44 (u8 wrap).
    let (n, ty, diags) = typed_stored(DISTINCT_SRC, "SUM");
    assert_eq!(n, 44);
    assert_eq!(ty, Ty::Newtype("Angle".into()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn newtype_distinctness_cross_type_is_an_error() {
    let (v, diags) = eval(DISTINCT_SRC, "MIX");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[cross-type mix]")
            && d.message.contains("Angle")
            && d.message.contains("Pos")),
        "expected a cross-type-mix naming both Angle and Pos, got {diags:?}"
    );
}

// ---- 10. Appendix A — parallax bitfield + struct layout & emission ------
//
// `bitfield Packed: u16 { op: 1, s2: 4, s1: 4 }` (9 of 16 bits used) and a
// `struct BandEntry (size: 10)`. Field order here is (fact_a, fact_b, cell_y,
// pad) rather than the Appendix A prose order (cell_y, fact_a, fact_b, ...):
// deliberately chosen so every 2-byte field lands at an EVEN offset, keeping
// this test about size/offset/packing rather than the separate
// `[layout.odd-field]` warning (the same "reorder to stay diagnostic-free"
// convention `eval_layout.rs`'s `sizeof_struct` test uses).
//
// Layout: fact_a: Packed (u16, 2B) @0, fact_b: Packed (u16, 2B) @2,
// cell_y: u8 (1B) @4, pad: [u8; 5] (5B) @5. Total = 2+2+1+5 = 10, matching the
// declared `(size: 10)`.
const BAND_ENTRY_SRC: &str = "\
module engine.parallax

bitfield Packed: u16 { op: 1, s2: 4, s1: 4 }

struct BandEntry (size: 10) {
    fact_a: Packed,
    fact_b: Packed,
    cell_y: u8,
    pad: [u8; 5],
}

const SZ = sizeof(BandEntry)
const OFF_A = offsetof(BandEntry, fact_a)
const OFF_B = offsetof(BandEntry, fact_b)
const OFF_Y = offsetof(BandEntry, cell_y)
const PACK = Packed{ op: 1, s2: 5, s1: 3 }

data B: BandEntry = BandEntry{
    fact_a: Packed{ op: 1, s2: 5, s1: 3 },
    fact_b: Packed{ op: 0, s2: 2, s1: 1 },
    cell_y: 7,
    pad: [0, 0, 0, 0, 0],
}
";

#[test]
fn band_entry_sizeof_and_offsets_match_the_declared_size() {
    assert_eq!(ok(BAND_ENTRY_SRC, "SZ"), int(10));
    assert_eq!(ok(BAND_ENTRY_SRC, "OFF_A"), int(0));
    assert_eq!(ok(BAND_ENTRY_SRC, "OFF_B"), int(2));
    assert_eq!(ok(BAND_ENTRY_SRC, "OFF_Y"), int(4));
}

#[test]
fn packed_bitfield_literal_packs_msb_to_lsb() {
    // Packed: op(1)@lsb15, s2(4)@lsb11, s1(4)@lsb7 (matches
    // `partial_bitfield_fits_without_filling` in eval_bitfields.rs — same
    // field shape). op=1, s2=5, s1=3 ->
    //   1<<15 | 5<<11 | 3<<7 = 32768 + 10240 + 384 = 43392.
    assert_eq!(ok(BAND_ENTRY_SRC, "PACK"), int(43392));
}

#[test]
fn band_entry_data_item_lowers_to_the_right_cells_at_the_right_widths() {
    let (file, diags) = parse_str(BAND_ENTRY_SRC);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (buf, diags) = eval_data(&file, "B");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 10);
    // fact_a: op=1,s2=5,s1=3 -> 43392 (as above).
    // fact_b: op=0,s2=2,s1=1 -> 0<<15 | 2<<11 | 1<<7 = 4096 + 128 = 4224.
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 43392, width: 2, signed: false },
            Cell::Scalar { value: 4224, width: 2, signed: false },
            Cell::Scalar { value: 7, width: 1, signed: false },
            Cell::Scalar { value: 0, width: 1, signed: false },
            Cell::Scalar { value: 0, width: 1, signed: false },
            Cell::Scalar { value: 0, width: 1, signed: false },
            Cell::Scalar { value: 0, width: 1, signed: false },
            Cell::Scalar { value: 0, width: 1, signed: false },
        ]
    );
}

#[test]
fn band_entry_declared_size_mismatch_names_fields_and_delta() {
    let src = "\
module engine.parallax

bitfield Packed: u16 { op: 1, s2: 4, s1: 4 }

struct BandEntryBad (size: 12) {
    fact_a: Packed,
    fact_b: Packed,
    cell_y: u8,
    pad: [u8; 5],
}
";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_struct(&file, "BandEntryBad");
    // The computed layout (10 bytes) is still surfaced even though it
    // disagrees with the declared 12 — the raw layout is not poisoned by a
    // size mismatch.
    assert_eq!(layout.expect("BandEntryBad should still lay out").size, 10);
    assert!(
        diags.iter().any(|d| {
            d.message.contains("declared size 12")
                && d.message.contains("fields total 10")
                && d.message.contains("fact_a @0")
                && d.message.contains("cell_y @4")
                && d.message.contains("off by 2")
                && d.message.contains("too small")
        }),
        "expected a size-mismatch diagnostic naming the fields and delta, got {diags:?}"
    );
}

// ---- 11. Appendix A — ParallaxConfig (size: 28): a second (size:)-checked
// struct --------------------------------------------------------------------

#[test]
fn parallax_config_sizeof_matches_the_declared_28_bytes() {
    // band_count: u8 (1B, default computed from a const) + reserved: [u8; 27]
    // (27B) = 28, matching the declared `(size: 28)`.
    let src = "\
module engine.parallax

const NUM_BANDS = 4

struct ParallaxConfig (size: 28) {
    band_count: u8 = NUM_BANDS,
    reserved: [u8; 27],
}

const SZ = sizeof(ParallaxConfig)
";
    assert_eq!(ok(src, "SZ"), int(28));
}

// ---- 12. Appendix B-ish — match dispatch over a payload-bearing enum -----

const TOKEN_SRC: &str = "\
module engine.fstring

comptime enum Token { Literal(int), Arg(int), End }

comptime fn opcode(t: Token) -> int {
    return match t {
        Literal(v) => v,
        Arg(v) => 100 + v,
        End => 255,
    }
}

const L = opcode(Token.Literal(7))
const A = opcode(Token.Arg(3))
const E = opcode(Token.End)
";

#[test]
fn exhaustive_match_dispatches_and_binds_each_payload() {
    assert_eq!(ok(TOKEN_SRC, "L"), int(7));
    assert_eq!(ok(TOKEN_SRC, "A"), int(103));
    assert_eq!(ok(TOKEN_SRC, "E"), int(255));
}

#[test]
fn non_exhaustive_match_over_the_same_enum_names_the_missing_variant() {
    // Same `Token` enum and dispatch shape as `TOKEN_SRC`, but the `End` arm
    // is dropped and there is no `_` wildcard.
    let src = "\
module engine.fstring

comptime enum Token { Literal(int), Arg(int), End }

comptime fn opcode_bad(t: Token) -> int {
    return match t {
        Literal(v) => v,
        Arg(v) => 100 + v,
    }
}

const BAD = opcode_bad(Token.End)
";
    let (_, diags) = eval(src, "BAD");
    assert!(
        diags.iter().any(|d| d.message.contains("[match.non-exhaustive]") && d.message.contains("End")),
        "expected a non-exhaustive diagnostic naming End, got {diags:?}"
    );
}

// ---- 13. integrated: functional glue + a per-element bitfield + the Data
// monoid, emitted through a `data` item ------------------------------------
//
// `bitfield Cell: u8 { flag: 1, idx: 7 }` (flag@lsb7, idx@lsb0 — 8 of 8 bits
// used). `build_table` maps each input int through a bitfield pack (a typed
// comptime construction) into a one-byte `Data` cell (`byte`), then folds the
// resulting array of `Data` values with the `++` monoid (T7) down to one
// buffer — proving the Plan 2 functional glue (`|>`/map/fold/lambda), typed
// bitfield construction (T4), and the Data monoid (T7) all compose through a
// `data` item.
//
// Hand-computed bytes for xs = [0, 1, 2, 3]:
//   x=0: flag=0%2=0, idx=0 -> 0<<7 | 0 = 0
//   x=1: flag=1%2=1, idx=1 -> 1<<7 | 1 = 129
//   x=2: flag=2%2=0, idx=2 -> 0<<7 | 2 = 2
//   x=3: flag=3%2=1, idx=3 -> 1<<7 | 3 = 131
#[test]
fn integrated_pipeline_builds_a_data_table_from_bitfield_packed_bytes() {
    let src = "\
module engine.parallax

bitfield Cell: u8 { flag: 1, idx: 7 }

comptime fn pack_row(i: int) -> int {
    return Cell{ flag: i % 2, idx: i }
}

comptime fn build_table(xs: [int; 4]) {
    return xs |> map(|x| byte(pack_row(x))) |> fold(Data.empty, |a, b| a ++ b)
}

data TABLE = build_table([0, 1, 2, 3])
";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (buf, diags) = eval_data(&file, "TABLE");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let buf = buf.expect("data buf");
    assert_eq!(buf.size, 4);
    assert_eq!(
        buf.cells,
        vec![
            Cell::Scalar { value: 0, width: 1, signed: false },
            Cell::Scalar { value: 129, width: 1, signed: false },
            Cell::Scalar { value: 2, width: 1, signed: false },
            Cell::Scalar { value: 131, width: 1, signed: false },
        ]
    );
}
