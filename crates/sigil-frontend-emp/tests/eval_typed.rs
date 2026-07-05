//! Integration tests for T5 (Spec 2, Plan 3): newtype distinctness + sized /
//! wrapping arithmetic + `fixed<I,F>` scale checking + `rescale`.
//!
//! The coexistence rule (D-P3.3): bare comptime `int` keeps the Plan-2
//! overflow-is-error behaviour; ONLY a value carrying a sized nominal type
//! (`Value::Typed`, produced by newtype construction / `fixed<>` mul / rescale)
//! wraps at its underlying width or scale. Each case parses a full `.emp` file,
//! evaluates a `const`, and asserts on the resulting value / diagnostics.
//!
//! NOTE: `.emp` hex literals use the `$` sigil (not `0x`); the Rust-side
//! assertions use ordinary `0x` literals.
use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::layout::Ty;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;

fn eval(src: &str, name: &str) -> (Option<Value>, Vec<sigil_span::Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_const(&file, name)
}

/// The stored int of a `const` that must evaluate cleanly to a `Value::Typed`.
fn typed_stored(src: &str, name: &str) -> (i128, Ty, Vec<sigil_span::Diagnostic>) {
    let (v, diags) = eval(src, name);
    match v {
        Some(Value::Typed { ty, val }) => {
            (val.as_stored_int().expect("typed wraps an int"), *ty, diags)
        }
        other => panic!("expected a Value::Typed, got {other:?} (diags {diags:?})"),
    }
}

// ---- prim-underlying newtypes: wrapping arithmetic ---------------------

#[test]
fn newtype_add_wraps_at_u8_width() {
    // Angle = u8; 200 + 100 = 300, wraps mod 256 = 44, still Typed(Angle).
    let src = "module m\nnewtype Angle = u8\nconst N = Angle(200) + Angle(100)\n";
    let (n, ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 44, "200 + 100 mod 256 = 44");
    assert_eq!(ty, Ty::Newtype("Angle".into()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn newtype_mul_wraps_at_u8_width() {
    // u8 multiply also wraps at 8 bits (D2.9): 20 * 20 = 400 mod 256 = 144.
    let src = "module m\nnewtype Angle = u8\nconst N = Angle(20) * Angle(20)\n";
    let (n, _ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 144, "20 * 20 = 400 mod 256 = 144");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn signed_newtype_wraps_two_complement() {
    // Delta = i8; 100 + 100 = 200 wraps into i8 as -56.
    let src = "module m\nnewtype Delta = i8\nconst N = Delta(100) + Delta(100)\n";
    let (n, ty, diags) = typed_stored(src, "N");
    assert_eq!(n, -56, "100 + 100 = 200 -> i8 = -56");
    assert_eq!(ty, Ty::Newtype("Delta".into()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn typed_plus_bare_literal_coerces_and_wraps() {
    // Typed + bare int literal: coerce the literal into Angle, wrap as u8.
    let src = "module m\nnewtype Angle = u8\nconst N = Angle(200) + 100\n";
    let (n, ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 44);
    assert_eq!(ty, Ty::Newtype("Angle".into()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    // ...and the symmetric bare-literal-on-the-left form.
    let src = "module m\nnewtype Angle = u8\nconst N = 100 + Angle(200)\n";
    let (n, _ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 44);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn cross_type_mix_of_distinct_newtypes_is_an_error() {
    let src = "module m\nnewtype Angle = u8\nnewtype Pos = u8\n\
               const N = Angle(10) + Pos(10)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[cross-type mix]")
            && d.message.contains("Angle")
            && d.message.contains("Pos")),
        "expected a cross-type-mix naming both types, got {diags:?}"
    );
}

// ---- comparisons on typed values ---------------------------------------

#[test]
fn same_type_comparison_yields_bool() {
    let src = "module m\nnewtype Angle = u8\nconst N = Angle(10) < Angle(20)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Bool(true)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let src = "module m\nnewtype Angle = u8\nconst N = Angle(20) == Angle(20)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Bool(true)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn cross_type_comparison_is_an_error() {
    let src = "module m\nnewtype Angle = u8\nnewtype Pos = u8\n\
               const N = Angle(10) == Pos(10)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[cross-type mix]")),
        "expected a cross-type-mix error, got {diags:?}"
    );
}

// ---- fixed<> scale rules (via newtype-over-fixed) ----------------------

#[test]
fn fixed_same_scale_add_is_transparent() {
    // newtype Fix16 = fixed<16,16>. Two Fix16 values add transparently: the
    // stored ints (already scaled) sum, staying Typed(Fix16). 1.5 + 2.5 as
    // 16.16: 0x18000 + 0x28000 = 0x40000 (== 4.0).
    let src = "module m\nnewtype Fix16 = fixed<16,16>\n\
               const N = Fix16($18000) + Fix16($28000)\n";
    let (n, ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 0x40000);
    assert_eq!(ty, Ty::Newtype("Fix16".into()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn fixed_multiply_doubles_the_scale() {
    // Fix16 * Fix16 -> bare fixed<32,32> (scale combined), stored ints multiply,
    // no wrap. 2.0 * 3.0 in 16.16: 0x20000 * 0x30000 = 0x600000000, which read
    // as 32.32 is 6.0.
    let src = "module m\nnewtype Fix16 = fixed<16,16>\n\
               const N = Fix16($20000) * Fix16($30000)\n";
    let (n, ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 0x20000i128 * 0x30000i128);
    assert_eq!(n, 0x6_0000_0000);
    assert_eq!(ty, Ty::Fixed { i: 32, f: 32 }, "scale doubled to fixed<32,32>");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn fixed_scale_mismatch_add_names_rescale() {
    // fixed<16,16> + fixed<8,8> is a scale mismatch — never a silent shift.
    let src = "module m\nnewtype Fix16 = fixed<16,16>\nnewtype Fix8 = fixed<8,8>\n\
               const N = Fix16($10000) + Fix8($100)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[scale mismatch]")
            && d.message.contains("rescale")),
        "expected a scale-mismatch error naming rescale, got {diags:?}"
    );
}

#[test]
fn doubled_scale_meeting_same_scale_is_a_scale_mismatch() {
    // A fixed<32,32> (from a multiply) added to a fixed<16,16> is the scale
    // mismatch that names rescale<16,16> (D2.10).
    let src = "module m\nnewtype Fix16 = fixed<16,16>\n\
               const N = (Fix16($20000) * Fix16($30000)) + Fix16($10000)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[scale mismatch]")
            && d.message.contains("rescale<16,16>")),
        "expected a scale-mismatch naming rescale<16,16>, got {diags:?}"
    );
}

// ---- rescale ------------------------------------------------------------

#[test]
fn rescale_narrows_a_doubled_scale() {
    // rescale<16,16> of a fixed<32,32> value shifts its stored int right by 16.
    // Fix16($20000) * Fix16($30000) = 0x600000000 as fixed<32,32>; rescaling
    // to fixed<16,16> shifts right 16 -> 0x60000 (== 6.0 in 16.16).
    let src = "module m\nnewtype Fix16 = fixed<16,16>\n\
               const N = rescale<16,16>(Fix16($20000) * Fix16($30000))\n";
    let (n, ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 0x6_0000_0000i128 >> 16);
    assert_eq!(n, 0x60000);
    assert_eq!(ty, Ty::Fixed { i: 16, f: 16 });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn rescale_result_is_a_bare_fixed_that_adds_at_common_scale() {
    // Two rescaled products are both BARE fixed<16,16> (same nominal type), so
    // they add transparently. 6.0 + 1.0 in 16.16 = 0x60000 + 0x10000 = 0x70000.
    let src = "module m\nnewtype Fix16 = fixed<16,16>\n\
               const N = rescale<16,16>(Fix16($20000) * Fix16($30000)) \
               + rescale<16,16>(Fix16($10000) * Fix16($10000))\n";
    let (n, ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 0x70000);
    assert_eq!(ty, Ty::Fixed { i: 16, f: 16 });
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn rescale_on_a_non_fixed_is_diagnosed() {
    let src = "module m\nnewtype Angle = u8\nconst N = rescale<16,16>(Angle(5))\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("rescale expects a fixed")),
        "expected a rescale-non-fixed error, got {diags:?}"
    );
}

// ---- newtype distinctness ----------------------------------------------

#[test]
fn newtype_over_fixed_vs_bare_fixed_is_cross_type() {
    // newtype Fix = fixed<16,16>; a rescale produces a BARE fixed<16,16>. Same
    // scale, distinct nominal type -> cross-type mix, NOT a scale mismatch.
    let src = "module m\nnewtype Fix = fixed<16,16>\n\
               const N = Fix($10000) + rescale<16,16>(Fix($10000) * Fix($10000))\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[cross-type mix]")),
        "expected a cross-type-mix (Fix vs bare fixed<16,16>), got {diags:?}"
    );
}

#[test]
fn newtype_over_fixed_same_type_add_is_transparent() {
    // A newtype over fixed inherits the fixed same-scale add: two Fix values add
    // transparently and stay Typed(Fix). 1.0 + 1.0 = 0x10000 + 0x10000 = 0x20000.
    let src = "module m\nnewtype Fix = fixed<16,16>\n\
               const N = Fix($10000) + Fix($10000)\n";
    let (n, ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 0x20000);
    assert_eq!(ty, Ty::Newtype("Fix".into()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- the coexistence guarantee: bare-int overflow STILL errors ---------

#[test]
fn bare_int_overflow_still_errors_not_wraps() {
    // Regression (D-P3.3): the wrapping introduced for typed values must NOT
    // leak into bare-int arithmetic. i128 overflow is still a diagnostic.
    // `1 << 126` is fine in i128; doubling it overflows (> 2^127 - 1).
    let src = "module m\nconst BIG = 1 << 126\nconst N = BIG + BIG\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("integer overflow")),
        "bare-int overflow must still error, got {diags:?}"
    );
}

#[test]
fn bare_int_arithmetic_is_unchanged() {
    // A plain bare-int sum stays an exact Int (no wrapping, no Typed).
    let src = "module m\nconst N = 200 + 100\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Int(300)));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

// ---- T5 review: silent-failure regressions -----------------------------

#[test]
fn typed_div_and_mod_by_zero_are_diagnosed_not_panics() {
    for op in ["/", "%"] {
        let src = format!("module m\nnewtype Angle = u8\nconst N = Angle(10) {op} Angle(0)\n");
        let (v, diags) = eval(&src, "N");
        assert_eq!(v, Some(Value::Poison), "op {op}");
        assert!(
            diags.iter().any(|d| d.message.contains("by zero")),
            "op {op}: expected a by-zero diagnostic, got {diags:?}"
        );
    }
}

#[test]
fn typed_valid_shift_works() {
    // Angle(5) << 2 = 20, still Typed(Angle), no diagnostic.
    let src = "module m\nnewtype Angle = u8\nconst N = Angle(5) << 2\n";
    let (n, ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 20);
    assert_eq!(ty, Ty::Newtype("Angle".into()));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn typed_out_of_range_shift_is_diagnosed() {
    // CRITICAL 2 repro: pre-fix these returned silent wrong answers (5, or a
    // no-op) with ZERO diagnostics. A count outside `0..width` must now error.
    // `<< 128` (past 128), `<< 8` (== the u8 width), and `<< -1` are all bad.
    for amount in ["128", "8", "(0 - 1)"] {
        let src = format!("module m\nnewtype Angle = u8\nconst N = Angle(5) << {amount}\n");
        let (v, diags) = eval(&src, "N");
        assert_eq!(v, Some(Value::Poison), "shift by {amount}");
        assert!(
            diags.iter().any(|d| d.message.contains("shift amount out of range")),
            "shift by {amount}: expected a shift-range diagnostic, got {diags:?}"
        );
    }
}

#[test]
fn small_fixed_construction_out_of_range_is_diagnosed() {
    // CRITICAL 1 repro: pre-fix `Fix(999999)` for an 8-bit fixed store was
    // silently accepted (the `_ => true` catch-all). fixed<4,4> is a signed
    // 8-bit store, range -128..=127.
    let src = "module m\nnewtype Fix = fixed<4,4>\nconst N = Fix(999999)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("999999 not in -128..127")),
        "expected an out-of-range fixed construction, got {diags:?}"
    );

    // ...and an in-range construction still works (1.0 in 4.4 is 0x10 = 16).
    let src = "module m\nnewtype Fix = fixed<4,4>\nconst N = Fix(16)\n";
    let (n, _ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 16);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn too_wide_fixed_is_diagnosed() {
    // review Minor #7: a fixed whose I+F >= 128 cannot be wrapped in the i128
    // domain — construction must reject it rather than silently skip wrapping.
    let src = "module m\nnewtype Huge = fixed<100,100>\nconst N = Huge(1)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[fixed.too-wide]")),
        "expected a fixed.too-wide diagnostic, got {diags:?}"
    );
}

#[test]
fn cyclic_newtype_through_arithmetic_is_diagnosed() {
    // IMPORTANT 3 repro: pre-fix `effective_underlying` returned a bare Poison
    // with ZERO diagnostics on a cycle. `A(x)` constructs (the `where` bound
    // bottoms out), but the arithmetic must resolve the underlying and detect
    // the `A -> A` cycle, reporting it like the sibling guards.
    let src = "module m\nnewtype A = A where 0..10\nconst N = A(5) + A(3)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic type")),
        "expected a cyclic-type diagnostic, got {diags:?}"
    );
}

#[test]
fn subtraction_respects_operand_order_in_both_coercion_directions() {
    // `5 - Angle(3)` = 2 (bare literal coerced on the left).
    let src = "module m\nnewtype Angle = u8\nconst N = 5 - Angle(3)\n";
    let (n, _ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 2);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    // `Angle(3) - 5` = -2, wraps into u8 as 254 (locks the non-commutative order).
    let src = "module m\nnewtype Angle = u8\nconst N = Angle(3) - 5\n";
    let (n, _ty, diags) = typed_stored(src, "N");
    assert_eq!(n, 254);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}
