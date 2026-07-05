//! Integration tests for the `.emp` types & layout engine (Spec 2, Plan 3 —
//! T2): the `Ty` model, `size_of_ty`, `layout_of_struct`, and `check_in_range`.
//! Each case parses a full `.emp` file (asserting a clean parse), then drives
//! the layout entry points and asserts on sizes/offsets and diagnostics.
use sigil_frontend_emp::ast::{Expr, Path, Type};
use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::layout::{
    check_in_range, check_value_fits_ty, layout_struct, layout_structs_shared, size_of_type,
};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;
use sigil_span::{SourceId, Span};

fn span() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

fn path(name: &str) -> Path {
    Path { segments: vec![name.to_string()], span: span() }
}

fn named(name: &str) -> Type {
    Type::Named(path(name))
}

/// Parse `src`, asserting a clean parse, returning the file.
fn parse(src: &str) -> sigil_frontend_emp::ast::File {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    file
}

// ---- size_of_ty on primitives and structural types --------------------

#[test]
fn size_of_primitives() {
    let file = parse("module m\n");
    for (ty, want) in [("u8", 1usize), ("i8", 1), ("u16", 2), ("i16", 2), ("u32", 4), ("i32", 4)]
    {
        let (sz, diags) = size_of_type(&file, &named(ty));
        assert_eq!(sz, want, "size of {ty}");
        assert!(diags.is_empty(), "unexpected diagnostics for {ty}: {diags:?}");
    }
}

#[test]
fn size_of_pointer_is_four() {
    let file = parse("module m\n");
    let (sz, diags) = size_of_type(&file, &Type::Ptr(Box::new(named("u16"))));
    assert_eq!(sz, 4);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn size_of_array() {
    let file = parse("module m\n");
    let ty = Type::Array(Box::new(named("u16")), Expr::Int(4, span()));
    let (sz, diags) = size_of_type(&file, &ty);
    assert_eq!(sz, 8);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn size_of_fixed() {
    let file = parse("module m\n");
    let (sz, diags) = size_of_type(&file, &Type::Fixed { i: 16, f: 16 });
    assert_eq!(sz, 4);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let (sz, diags) = size_of_type(&file, &Type::Fixed { i: 8, f: 8 });
    assert_eq!(sz, 2);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn size_of_newtype_follows_underlying() {
    let file = parse("module m\nnewtype Angle = u8\n");
    let (sz, diags) = size_of_type(&file, &named("Angle"));
    assert_eq!(sz, 1);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn size_of_enum_follows_repr() {
    let file = parse("module m\nenum Anim: u8 { Idle = 0, Seed = 1 }\n");
    let (sz, diags) = size_of_type(&file, &named("Anim"));
    assert_eq!(sz, 1);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn unknown_type_is_poison_with_diagnostic() {
    let file = parse("module m\n");
    let (sz, diags) = size_of_type(&file, &named("Nope"));
    assert_eq!(sz, 0);
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(diags[0].message.contains("unknown type"), "was {:?}", diags[0].message);
}

// ---- layout_of_struct: offsets and total size --------------------------

#[test]
fn struct_layout_offsets_no_padding() {
    // Declaration-order, next-byte packing: a@0, b@1, c@3, size 7. b (2-byte)
    // and c (4-byte) both land at odd offsets — exactly the no-implicit-padding
    // scenario T3's `[layout.odd-field]` lint warns about (§4.3); this is a
    // default-on WARNING, not an error, and doesn't affect the computed layout.
    let file = parse("module m\nstruct S { a: u8, b: u16, c: u32 }\n");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(
        diags.iter().all(|d| d.level == sigil_span::Level::Warning),
        "expected only odd-field WARNINGs (T3), got {diags:?}"
    );
    assert_eq!(diags.len(), 2, "expected b and c to both warn odd-field, got {diags:?}");
    let layout = layout.expect("S should lay out");
    assert_eq!(layout.size, 7);
    assert_eq!(layout.fields.len(), 3);
    assert_eq!(layout.fields[0].offset, 0);
    assert_eq!(layout.fields[1].offset, 1);
    assert_eq!(layout.fields[2].offset, 3);
    assert_eq!(layout.fields[0].size, 1);
    assert_eq!(layout.fields[1].size, 2);
    assert_eq!(layout.fields[2].size, 4);
}

#[test]
fn by_value_self_reference_is_cyclic_not_a_hang() {
    // A struct containing itself by value has infinite size — must be reported,
    // not overflow the stack.
    let file = parse("module m\nstruct Node { next: Node }\n");
    let (_layout, diags) = layout_struct(&file, "Node");
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic struct layout")),
        "expected a cyclic-layout diagnostic, got {diags:?}"
    );
}

#[test]
fn by_pointer_self_reference_is_finite() {
    // `*Node` is a pointer (size 4) and does not recurse into the pointee.
    let file = parse("module m\nstruct Node { next: *Node }\n");
    let (layout, diags) = layout_struct(&file, "Node");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let layout = layout.expect("Node should lay out");
    assert_eq!(layout.size, 4);
    assert_eq!(layout.fields[0].offset, 0);
    assert_eq!(layout.fields[0].size, 4);
}

#[test]
fn mutual_struct_cycle_poisons_layout_not_just_size() {
    // Regression (Critical 1): the outer frame must NOT overwrite the poisoned
    // layout a deeper call memoized. The returned layout must be the poisoned
    // zero one — empty fields, size 0 — not a numerically-wrong `{size:2, ...}`.
    let file = parse("module m\nstruct A { pad: u16, b: B }\nstruct B { a: A }\n");
    let (layout, diags) = layout_struct(&file, "A");
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic struct layout")),
        "expected a cyclic-layout diagnostic, got {diags:?}"
    );
    let layout = layout.expect("A should return a (poisoned) layout");
    assert_eq!(layout.size, 0, "poisoned layout must be size 0, was {}", layout.size);
    assert!(
        layout.fields.is_empty(),
        "poisoned layout must have no fields, had {:?}",
        layout.fields
    );
}

#[test]
fn shared_evaluator_poisons_every_struct_on_the_cycle() {
    // Regression (forward-looking): on a SHARED evaluator, laying out the entry
    // struct of a cycle must poison EVERY member — so a later direct query for a
    // "middle" struct returns the poison, not a stale wrong finite layout. (This
    // is the shape T3's per-struct `(size: N)` verification will drive.)
    let file = parse("module m\nstruct A { b: B }\nstruct B { a: A }\n");
    let (layouts, diags) = layout_structs_shared(&file, &["A", "B"]);
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic struct layout")),
        "expected a cyclic-layout diagnostic, got {diags:?}"
    );
    // Exactly one chain diagnostic — not one per cycle member.
    assert_eq!(
        diags.iter().filter(|d| d.message.contains("cyclic struct layout")).count(),
        1,
        "expected a single chain diagnostic, got {diags:?}"
    );
    // A (the entry) is poisoned.
    let a = layouts[0].clone().expect("A should return a layout");
    assert_eq!(a.size, 0);
    assert!(a.fields.is_empty(), "A must have no fields, had {:?}", a.fields);
    // B (queried directly on the SAME evaluator) is also poisoned — the fields
    // are the tell: pre-fix, B memoized as `{size:0, fields:[a@0]}` (a lie).
    let b = layouts[1].clone().expect("B should return a layout");
    assert_eq!(b.size, 0, "B must be poisoned, was size {}", b.size);
    assert!(b.fields.is_empty(), "B must have no fields, had {:?}", b.fields);
}

#[test]
fn newtype_cycle_is_diagnosed_not_a_stack_overflow() {
    // Regression (Critical 2): a `newtype A = B; newtype B = A` cycle never
    // passes through a struct hop, so it must be caught by the newtype guard.
    let file = parse("module m\nnewtype A = B\nnewtype B = A\n");
    let (sz, diags) = size_of_type(&file, &named("A"));
    assert_eq!(sz, 0);
    assert!(
        diags.iter().any(|d| d.message.contains("cyclic type")),
        "expected a cyclic-type diagnostic, got {diags:?}"
    );
}

#[test]
fn fixed_non_byte_multiple_is_diagnosed_not_a_panic() {
    // Regression (fixed sizing): `fixed<1,2>` = 3 bits, not a whole byte.
    let file = parse("module m\n");
    let (sz, diags) = size_of_type(&file, &Type::Fixed { i: 1, f: 2 });
    // Best-effort ceil (3 bits -> 1 byte), plus a diagnostic.
    assert_eq!(sz, 1);
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(
        diags[0].message.contains("not a whole number of bytes"),
        "was {:?}",
        diags[0].message
    );
}

#[test]
fn oversized_array_length_is_diagnosed_not_truncated() {
    // Regression (Important 3): a huge in-i128 length must not silently truncate
    // through `as usize` — it must diagnose and poison (size 0).
    let file = parse("module m\n");
    // 2^100, far beyond usize but well within i128.
    let big = Expr::Binary {
        op: sigil_frontend_emp::ast::BinOp::Shl,
        lhs: Box::new(Expr::Int(1, span())),
        rhs: Box::new(Expr::Int(100, span())),
        span: span(),
    };
    let ty = Type::Array(Box::new(named("u8")), big);
    let (sz, diags) = size_of_type(&file, &ty);
    assert_eq!(sz, 0);
    assert!(
        diags.iter().any(|d| d.message.contains("too large")),
        "expected an oversized-length diagnostic, got {diags:?}"
    );
}

// ---- check_in_range: inclusive on BOTH ends ----------------------------

#[test]
fn check_in_range_inclusive_bounds() {
    let (ok, diags) = check_in_range(5, 0, 63);
    assert!(ok);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    // Inclusive hi.
    let (ok, diags) = check_in_range(63, 0, 63);
    assert!(ok);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    // Inclusive lo.
    let (ok, diags) = check_in_range(0, 0, 63);
    assert!(ok);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    // Above the range.
    let (ok, diags) = check_in_range(64, 0, 63);
    assert!(!ok);
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
    assert!(diags[0].message.contains("64 not in 0..63"), "was {:?}", diags[0].message);

    // Below the range.
    let (ok, diags) = check_in_range(-1, 0, 63);
    assert!(!ok);
    assert_eq!(diags.len(), 1, "expected one diagnostic, got {diags:?}");
}

// ---- check_value_fits_ty: the shared refinement mechanism (T4) --------

#[test]
fn check_value_fits_ty_refined_newtype_boundaries() {
    let file = parse("module m\nnewtype PaletteLine = u8 where 0..63\n");
    let (ok, diags) = check_value_fits_ty(&file, &named("PaletteLine"), 40);
    assert!(ok, "40 should fit PaletteLine (0..63): {diags:?}");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");

    let (ok, diags) = check_value_fits_ty(&file, &named("PaletteLine"), 63);
    assert!(ok, "63 (inclusive hi) should fit PaletteLine: {diags:?}");
    assert!(diags.is_empty());

    let (ok, diags) = check_value_fits_ty(&file, &named("PaletteLine"), 70);
    assert!(!ok);
    assert!(
        diags.iter().any(|d| d.message.contains("70 not in 0..63")),
        "was {diags:?}"
    );
}

#[test]
fn check_value_fits_ty_bare_primitive_boundaries() {
    let file = parse("module m\n");
    let (ok, diags) = check_value_fits_ty(&file, &named("u8"), 200);
    assert!(ok, "200 fits u8: {diags:?}");
    assert!(diags.is_empty());

    let (ok, diags) = check_value_fits_ty(&file, &named("u8"), 300);
    assert!(!ok);
    assert!(diags.iter().any(|d| d.message.contains("300 not in 0..255")), "was {diags:?}");

    let (ok, diags) = check_value_fits_ty(&file, &named("i8"), -128);
    assert!(ok, "-128 fits i8: {diags:?}");
    let (ok, diags) = check_value_fits_ty(&file, &named("i8"), -129);
    assert!(!ok);
    assert!(diags.iter().any(|d| d.message.contains("-129 not in -128..127")), "was {diags:?}");
}

// ---- newtype/refined construction: Name(x) (T4) ------------------------

fn eval_helper(src: &str, name: &str) -> (Option<Value>, Vec<sigil_span::Diagnostic>) {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "expected a clean parse, got {diags:?}");
    eval_const(&file, name)
}

#[test]
fn newtype_construction_in_range_produces_a_typed_value() {
    // T5: construction now yields a `Value::Typed` carrying the nominal newtype,
    // wrapping the checked stored int (was the erased bare `Int` in T4).
    let src = "module m\nnewtype PaletteLine = u8 where 0..63\nconst N = PaletteLine(40)\n";
    let (v, diags) = eval_helper(src, "N");
    assert_eq!(v.as_ref().and_then(Value::as_stored_int), Some(40));
    assert!(
        matches!(&v, Some(Value::Typed { .. })),
        "expected a Value::Typed, got {v:?}"
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}

#[test]
fn newtype_construction_out_of_range_is_diagnosed() {
    let src = "module m\nnewtype PaletteLine = u8 where 0..63\nconst N = PaletteLine(70)\n";
    let (v, diags) = eval_helper(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("70 not in 0..63")),
        "was {diags:?}"
    );
}

#[test]
fn newtype_without_where_still_checked_against_its_underlying_primitive() {
    let src = "module m\nnewtype Angle = u8\nconst N = Angle(300)\n";
    let (v, diags) = eval_helper(src, "N");
    assert_eq!(v, Some(Value::Poison));
    assert!(
        diags.iter().any(|d| d.message.contains("300 not in 0..255")),
        "was {diags:?}"
    );
    // The diagnostic names the newtype the author wrote (`Angle`), not the
    // underlying `u8` — the minor-review fix.
    assert!(
        diags.iter().any(|d| d.message.contains("Angle")),
        "expected the message to name the newtype `Angle`, got {diags:?}"
    );

    let src = "module m\nnewtype Angle = u8\nconst N = Angle(200)\n";
    let (v, diags) = eval_helper(src, "N");
    assert_eq!(v.as_ref().and_then(Value::as_stored_int), Some(200));
    assert!(matches!(&v, Some(Value::Typed { .. })), "expected a Value::Typed, got {v:?}");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
}
