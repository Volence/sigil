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
