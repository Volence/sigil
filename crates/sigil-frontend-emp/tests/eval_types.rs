//! Integration tests for the `.emp` types & layout engine (Spec 2, Plan 3 —
//! T2): the `Ty` model, `size_of_ty`, `layout_of_struct`, and `check_in_range`.
//! Each case parses a full `.emp` file (asserting a clean parse), then drives
//! the layout entry points and asserts on sizes/offsets and diagnostics.
use sigil_frontend_emp::ast::{Expr, Path, Type};
use sigil_frontend_emp::layout::{check_in_range, layout_struct, size_of_type};
use sigil_frontend_emp::parse_str;
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
    // Declaration-order, next-byte packing: a@0, b@1, c@3, size 7.
    let file = parse("module m\nstruct S { a: u8, b: u16, c: u32 }\n");
    let (layout, diags) = layout_struct(&file, "S");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
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
