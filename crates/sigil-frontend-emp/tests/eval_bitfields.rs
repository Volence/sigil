//! Integration tests for the `.emp` bitfield layout + construction/packing
//! mechanism, and the enum-cast dispatch that leans on the same shared
//! refinement machinery (Spec 2, Plan 3 — T4).
//!
//! Layout cases drive [`layout_bitfield`] directly; construction/packing and
//! enum-cast cases go through a full parse + [`eval_const`], matching the
//! convention `eval_layout.rs` established for T3.
use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::layout::layout_bitfield;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;
use sigil_span::Diagnostic;

/// Parse `src` (asserting a clean parse) and evaluate the const named `name`.
fn eval(src: &str, name: &str) -> (Option<Value>, Vec<Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_const(&file, name)
}

fn int(n: i128) -> Value {
    Value::Int(n)
}

// ---- bitfield layout: MSB->LSB field placement -------------------------

#[test]
fn full_bitfield_fills_sequentially_msb_to_lsb() {
    // The Genesis art_tile word, ground truth: pri(1) pal(2) flip(2) tile(11),
    // no anchors, fields exactly fill the 16-bit repr.
    let src = "module m\nbitfield ArtTile: u16 { pri: 1, pal: 2, flip: 2, tile: 11 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_bitfield(&file, "ArtTile");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let layout = layout.expect("ArtTile should lay out");
    assert_eq!(layout.repr_bits, 16);
    let lsb = |name: &str| layout.fields.iter().find(|f| f.name == name).unwrap().lsb;
    assert_eq!(lsb("pri"), 15);
    assert_eq!(lsb("pal"), 13);
    assert_eq!(lsb("flip"), 11);
    assert_eq!(lsb("tile"), 0);
}

#[test]
fn partial_bitfield_fits_without_filling() {
    // 1 + 4 + 4 = 9 bits used of 16 — widths need NOT sum to repr_bits.
    let src = "module m\nbitfield Packed: u16 { op: 1, s2: 4, s1: 4 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_bitfield(&file, "Packed");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let layout = layout.expect("Packed should lay out");
    assert_eq!(layout.repr_bits, 16);
    let lsb = |name: &str| layout.fields.iter().find(|f| f.name == name).unwrap().lsb;
    assert_eq!(lsb("op"), 15);
    assert_eq!(lsb("s2"), 11);
    assert_eq!(lsb("s1"), 7);
}

#[test]
fn oversized_bitfield_fields_report_overflow() {
    // 5 + 5 = 10 bits, but the repr is only u8 (8 bits) — must not fit.
    let src = "module m\nbitfield Big: u8 { a: 5, b: 5 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_bitfield(&file, "Big");
    assert!(
        diags.iter().any(|d| d.message.contains("[bitfield.overflow]")),
        "expected a bitfield.overflow diagnostic, got {diags:?}"
    );
}

#[test]
fn anchor_places_field_at_explicit_lsb() {
    // `tile: 11 @ 0` is anchored at bit 0, leaving bits 12-11 unused (pal ends
    // at lsb 13, so the gap between pal's top (bit 14) and tile's top (bit 10)
    // is never assigned to any field).
    let src = "module m\nbitfield ArtTile: u16 { pri: 1, pal: 2, tile: 11 @ 0 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_bitfield(&file, "ArtTile");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let layout = layout.expect("ArtTile should lay out");
    let lsb = |name: &str| layout.fields.iter().find(|f| f.name == name).unwrap().lsb;
    assert_eq!(lsb("pri"), 15);
    assert_eq!(lsb("pal"), 13);
    assert_eq!(lsb("tile"), 0);
}

#[test]
fn anchored_field_exceeding_repr_is_out_of_range() {
    let src = "module m\nbitfield Bad: u8 { tile: 4 @ 6 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_bitfield(&file, "Bad");
    assert!(
        diags.iter().any(|d| d.message.contains("[bitfield.field-out-of-range]")),
        "expected a bitfield.field-out-of-range diagnostic, got {diags:?}"
    );
}

#[test]
fn overlapping_fields_are_diagnosed() {
    let src = "module m\nbitfield Bad: u8 { a: 4 @ 4, b: 4 @ 2 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (_layout, diags) = layout_bitfield(&file, "Bad");
    assert!(
        diags.iter().any(|d| d.message.contains("[bitfield.field-overlap]")),
        "expected a bitfield.field-overlap diagnostic, got {diags:?}"
    );
}

// ---- bitfield construction / packing -----------------------------------

#[test]
fn bitfield_literal_packs_fields_by_lsb() {
    // pri(1)@15, pal(2)@13, flip(2)@11, tile(11)@0.
    // 1<<15 | 2<<13 | 0<<11 | 5<<0 = 32768 + 16384 + 0 + 5 = 49157.
    let src = "module m\n\
               bitfield ArtTile: u16 { pri: 1, pal: 2, flip: 2, tile: 11 }\n\
               const N = ArtTile{ pri: 1, pal: 2, flip: 0, tile: 5 }\n";
    let (v, diags) = eval(src, "N");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(v, Some(int(49157)));
}

#[test]
fn bitfield_literal_field_out_of_range_is_diagnosed() {
    // pal is 2 bits wide (max 3); 5 does not fit.
    let src = "module m\n\
               bitfield ArtTile: u16 { pri: 1, pal: 2, flip: 2, tile: 11 }\n\
               const N = ArtTile{ pal: 5 }\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("5 not in 0..3")),
        "expected a range diagnostic, got {diags:?}"
    );
}

#[test]
fn bitfield_literal_omitted_field_defaults_to_zero() {
    // Only `tile` supplied; pri/pal/flip default to 0.
    let src = "module m\n\
               bitfield ArtTile: u16 { pri: 1, pal: 2, flip: 2, tile: 11 }\n\
               const N = ArtTile{ tile: 7 }\n";
    let (v, diags) = eval(src, "N");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(v, Some(int(7)));
}

#[test]
fn bitfield_literal_unknown_field_is_diagnosed() {
    let src = "module m\n\
               bitfield ArtTile: u16 { pri: 1, pal: 2, flip: 2, tile: 11 }\n\
               const N = ArtTile{ nope: 1 }\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("nope")),
        "expected an unknown-field diagnostic, got {diags:?}"
    );
}

// ---- enum casts ----------------------------------------------------------

#[test]
fn enum_cast_of_a_declared_value_yields_the_variant() {
    let src = "module m\n\
               enum Anim: u8 { Idle = 0, Seed = 1, Shoot = 2 }\n\
               const N = Anim(1)\n";
    let (v, diags) = eval(src, "N");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        v,
        Some(Value::Enum { ty_name: "Anim".to_string(), variant: "Seed".to_string(), payload: vec![] })
    );
}

#[test]
fn enum_cast_out_of_range_is_diagnosed() {
    let src = "module m\n\
               enum Anim: u8 { Idle = 0, Seed = 1, Shoot = 2 }\n\
               const N = Anim(9)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("[enum.out-of-range]")),
        "expected an enum.out-of-range diagnostic, got {diags:?}"
    );
    assert!(
        diags.iter().any(|d| d.message.contains('9') && d.message.contains("Anim")),
        "expected the message to name the value and the enum, got {diags:?}"
    );
}

#[test]
fn enum_cast_arg_return_does_not_leak_a_spurious_out_of_range() {
    // Regression (Critical): a `return` fired inside the cast argument belongs
    // to the CALLER. Before the fix, `eval_single_arg` did not bail on a leaked
    // `pending_return`, so the per-variant discriminant evals short-circuited to
    // Poison, no variant matched, and a spurious `[enum.out-of-range]` fired even
    // though the fn correctly returned 42.
    let src = "module m\n\
               enum Anim: u8 { Idle = 0, Seed = 1, Shoot = 2 }\n\
               comptime fn pick(c: int) -> int {\n\
                   let x = Anim(if c > 0 { return 42 } else { 1 })\n\
                   0\n\
               }\n\
               const R = pick(1)\n";
    let (v, diags) = eval(src, "R");
    assert_eq!(v, Some(int(42)), "the caller's return value must win");
    assert!(
        !diags.iter().any(|d| d.message.contains("[enum.out-of-range]")),
        "a leaked return must not fabricate an out-of-range diagnostic, got {diags:?}"
    );
}

// ---- enum auto-increment / mixed / duplicate discriminants -------------

#[test]
fn enum_cast_uses_auto_incremented_discriminants() {
    // A=5, B (auto → 6), C=1 (explicit reset), D (auto → 2). E(6) → B; E(1) → C.
    let src = "module m\nenum E: u8 { A = 5, B, C = 1, D }\n";
    let (bv, diags) = eval(&format!("{src}const N = E(6)\n"), "N");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        bv,
        Some(Value::Enum { ty_name: "E".to_string(), variant: "B".to_string(), payload: vec![] })
    );
    // C=1 and D=2 both live; E(1) → C, E(2) → D.
    let (cv, _) = eval(&format!("{src}const N = E(1)\n"), "N");
    assert_eq!(
        cv,
        Some(Value::Enum { ty_name: "E".to_string(), variant: "C".to_string(), payload: vec![] })
    );
    let (dv, _) = eval(&format!("{src}const N = E(2)\n"), "N");
    assert_eq!(
        dv,
        Some(Value::Enum { ty_name: "E".to_string(), variant: "D".to_string(), payload: vec![] })
    );
}

#[test]
fn enum_cast_duplicate_discriminant_resolves_first_match_wins() {
    // A=1 and B=1 collide; the cast must resolve to the FIRST declared (A),
    // deterministically.
    let src = "module m\nenum E: u8 { A = 1, B = 1 }\nconst N = E(1)\n";
    let (v, diags) = eval(src, "N");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        v,
        Some(Value::Enum { ty_name: "E".to_string(), variant: "A".to_string(), payload: vec![] })
    );
}

// ---- newtype construction cycle (through the real eval_call path) ------

#[test]
fn newtype_construction_cycle_is_diagnosed_not_a_crash() {
    // `newtype A = B; newtype B = A` has no scalar bound to bottom out at;
    // `check_value_fits_ty` must catch the cycle (via layout_in_progress) rather
    // than recurse forever. Exercised through construction, not just sizing.
    let src = "module m\nnewtype A = B\nnewtype B = A\nconst N = A(5)\n";
    let (v, diags) = eval(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic type")),
        "expected a cyclic-type diagnostic, got {diags:?}"
    );
}

// ---- bitfield layout memo: one diagnostic per malformed bitfield -------

#[test]
fn malformed_bitfield_reused_reports_overlap_once() {
    // A bitfield with an overlapping anchor, used in THREE literals within one
    // eval_const, must emit its overlap diagnostic exactly once (memoized).
    let src = "module m\n\
               bitfield Bad: u8 { a: 4 @ 4, b: 4 @ 2 }\n\
               const X = Bad{ a: 1 }\n\
               const Y = Bad{ a: 2 }\n\
               const Z = Bad{ a: 3 }\n\
               const N = X + Y + Z\n";
    let (_v, diags) = eval(src, "N");
    let overlaps =
        diags.iter().filter(|d| d.message.contains("[bitfield.field-overlap]")).count();
    assert_eq!(overlaps, 1, "expected exactly one overlap diagnostic, got {diags:?}");
}

// ---- anchor continuation: unanchored field after an anchored one -------

#[test]
fn unanchored_field_after_anchor_continues_from_reset_cursor() {
    // hi(2) fills MSB → lsb 6. mid(4) @ 0 anchors at bit 0 and resets cursor to
    // 0. lo(?) after it would underflow — instead use an anchored middle then a
    // trailing unanchored field ABOVE it via a fresh cursor is impossible, so
    // model the documented reset path: an anchored field lowers the cursor, and
    // a following unanchored field packs below the (reset) cursor.
    // Layout: top(3) fills from MSB → lsb 13. anchored(4) @ 4 → lsb 4, cursor→4.
    // tail(4) unanchored → lsb = cursor(4) - 4 = 0.
    let src = "module m\nbitfield B: u16 { top: 3, anchored: 4 @ 4, tail: 4 }\n";
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    let (layout, diags) = layout_bitfield(&file, "B");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let layout = layout.expect("B should lay out");
    let lsb = |name: &str| layout.fields.iter().find(|f| f.name == name).unwrap().lsb;
    assert_eq!(lsb("top"), 13);
    assert_eq!(lsb("anchored"), 4);
    assert_eq!(lsb("tail"), 0);
}
