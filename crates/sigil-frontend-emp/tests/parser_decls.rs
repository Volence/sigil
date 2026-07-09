use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::parse_str;

/// Parse and demand zero diagnostics.
fn ok(src: &str) -> File {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "diagnostics: {diags:?}");
    file
}

#[test]
fn module_header_plain_and_in_section() {
    let f = ok("module badniks.pitcher_plant\n");
    assert_eq!(f.module.path.segments, vec!["badniks", "pitcher_plant"]);
    assert_eq!(f.module.in_section, None);

    let f = ok("module badniks.pitcher_plant in obj_bank\n");
    assert_eq!(f.module.in_section.as_deref(), Some("obj_bank"));
}

#[test]
fn path_in_error_position_has_non_inverted_span() {
    // Regression (B2): when `path()` starts on a non-ident token, its opening
    // `expect_ident` consumes nothing and `prev_span().end` can precede the
    // start, producing an INVERTED span (end < start) that trips span-asserting
    // renderers. `module  = x` reaches the module path with a missing name and a
    // byte gap, so the span must still satisfy `start <= end`.
    let (f, diags) = parse_str("module  = x\n");
    assert!(!diags.is_empty(), "expected a parse error for the missing module name");
    let span = f.module.path.span;
    assert!(
        span.start <= span.end,
        "module path span is inverted: {}..{}",
        span.start,
        span.end
    );
}

#[test]
fn use_decls() {
    let f = ok("module m\nuse engine.objects.{Sst, Draw_Sprite}\nuse engine.gfx.*\nuse player.Player_1\n");
    assert_eq!(f.items.len(), 3);
    let Item::Use(u) = &f.items[0] else { panic!() };
    assert_eq!(u.base.segments, vec!["engine", "objects"]);
    assert_eq!(u.names, UseNames::List(vec!["Sst".into(), "Draw_Sprite".into()]));
    let Item::Use(u) = &f.items[1] else { panic!() };
    assert_eq!(u.names, UseNames::Glob);
    let Item::Use(u) = &f.items[2] else { panic!() };
    assert_eq!(u.names, UseNames::Whole);
    assert_eq!(u.base.segments, vec!["player", "Player_1"]);
}

#[test]
fn module_level_attributes() {
    let f = ok("module m\n@as_compat\n@allow(naming.pascal)\nuse engine.gfx.*\n");
    assert_eq!(f.attrs.len(), 2);
    assert_eq!(f.attrs[0].name, "as_compat");
    assert_eq!(f.attrs[1].args.len(), 1);
    assert_eq!(f.items.len(), 1);
}

#[test]
fn missing_module_header_is_diagnosed() {
    let (_, diags) = parse_str("const X: u8 = 1\n");
    assert!(!diags.is_empty());
}

#[test]
fn decl_spans_cover_whole_declaration() {
    let src = "module badniks.pitcher_plant in obj_bank\n";
    let f = ok(src);
    assert_eq!(f.module.span.start, 0);
    assert_eq!(f.module.span.end as usize, src.trim_end().len());
}

#[test]
fn pub_use_is_diagnosed() {
    let (_, diags) = parse_str("module m\npub use engine.gfx.*\n");
    assert_eq!(diags.len(), 1);
}

#[test]
fn empty_attr_args() {
    let f = ok("module m\n@as_compat()\nuse engine.gfx.*\n");
    assert_eq!(f.attrs[0].args.len(), 0);
}

#[test]
fn const_decls() {
    let f = ok("module m\nconst WAIT_TIME: u8 = 64\npub const TAU: float = 6.283185307179586\nconst Def = ObjDef{ code: init }\n");
    let Item::Const(c) = &f.items[0] else { panic!() };
    assert_eq!(c.name, "WAIT_TIME");
    assert!(!c.public);
    assert!(matches!(c.value, Expr::Int(64, _)));
    let Item::Const(c) = &f.items[1] else { panic!() };
    assert!(c.public);
    let Item::Const(c) = &f.items[2] else { panic!() };
    assert!(c.ty.is_none()); // inferred from ObjDef{..}
}

#[test]
fn equ_decls() {
    // `equ` (R-T0.2): an assembler equate item — mirrors `const`'s grammar
    // shape (name = expr, `pub` toggles module visibility) but is a distinct
    // AST node from Item::Const (its whole purpose is link-symbol emission,
    // Task 3 — not overloading `pub const`).
    let f = ok("module m\nequ FOO = 42\npub equ BAR = 7\n");
    let Item::Equ(e) = &f.items[0] else { panic!("expected Item::Equ, got {:?}", f.items[0]) };
    assert_eq!(e.name, "FOO");
    assert!(!e.is_pub);
    assert!(matches!(e.value, Expr::Int(42, _)));
    let Item::Equ(e) = &f.items[1] else { panic!("expected Item::Equ, got {:?}", f.items[1]) };
    assert_eq!(e.name, "BAR");
    assert!(e.is_pub);
}

#[test]
fn equ_is_reserved_as_a_declaration_name() {
    // `equ` used as an item's NAME (e.g. a const named `equ`) is now a parse
    // error — the new keyword must not silently shadow as an identifier.
    let (_, diags) = parse_str("module m\nconst equ = 1\n");
    assert!(!diags.is_empty(), "expected a diagnostic for `equ` as a const name");
    assert!(
        diags.iter().any(|d| d.message.contains("equ") && d.message.contains("reserved")),
        "diagnostic should name `equ` as reserved, got: {diags:?}"
    );
}

#[test]
fn enum_decl() {
    let f = ok("module m\nenum Anim: u8 { Idle = 0, Seed = 1, Shoot = 2 }\n");
    let Item::Enum(e) = &f.items[0] else { panic!() };
    assert_eq!(e.name, "Anim");
    assert_eq!(e.variants.len(), 3);
    assert_eq!(e.variants[2].name, "Shoot");
}

#[test]
fn types_parse() {
    let f = ok("module m\nconst A: *Sst = x\nconst B: [i8; 256] = y\nconst C: (Data, Code) = z\n");
    let Item::Const(c) = &f.items[0] else { panic!() };
    assert!(matches!(c.ty, Some(Type::Ptr(_))));
    let Item::Const(c) = &f.items[1] else { panic!() };
    assert!(matches!(c.ty, Some(Type::Array(_, _))));
    let Item::Const(c) = &f.items[2] else { panic!() };
    assert!(matches!(c.ty, Some(Type::Tuple(_))));
}

#[test]
fn use_base_span_excludes_names() {
    let src = "module m\nuse engine.objects.{Sst, Draw_Sprite}\n";
    let f = ok(src);
    let Item::Use(u) = &f.items[0] else { panic!() };
    assert_eq!(&src[u.base.span.start as usize..u.base.span.end as usize], "engine.objects");
}

#[test]
fn deep_pointer_type_is_an_error_not_an_abort() {
    let stars = "*".repeat(5_000);
    let (_, diags) = parse_str(&format!("module m\nconst X: {stars}u8 = 1\n"));
    assert!(!diags.is_empty());
}

#[test]
fn missing_module_header_still_parses_items() {
    // follow-up: with const_decl implemented, the parser must recover past a
    // missing module header and still parse the items.
    let (f, diags) = parse_str("const X: u8 = 1\n");
    assert!(!diags.is_empty());
    assert!(matches!(&f.items[0], Item::Const(c) if c.name == "X"));
}

#[test]
fn bitfield_decl() {
    let f = ok("module m\nbitfield ArtTile: u16 { pri: 1, pal: 2, tile: 11 @ 0 }\n");
    let Item::Bitfield(b) = &f.items[0] else { panic!() };
    assert_eq!(b.fields.len(), 3);
    assert_eq!(b.fields[2].bits, 11);
    assert_eq!(b.fields[2].anchor, Some(0));
    assert_eq!(b.fields[0].anchor, None);
}

#[test]
fn huge_bit_width_is_diagnosed_not_truncated() {
    let (_, diags) = parse_str("module m\nbitfield B: u16 { f: 5000000000 }\n");
    assert!(!diags.is_empty());
    let (_, diags) = parse_str("module m\nbitfield B: u16 { f: 1 @ 5000000000 }\n");
    assert!(!diags.is_empty());
}

#[test]
fn missing_bit_width_does_not_drop_next_field() {
    let (f, diags) = parse_str("module m\nbitfield B: u16 { f: , g: 2 }\n");
    assert!(!diags.is_empty());
    let Item::Bitfield(b) = &f.items[0] else { panic!() };
    assert_eq!(b.fields.len(), 2);
    assert_eq!(b.fields[1].name, "g");
}

#[test]
fn struct_decl_with_size_offsets_defaults() {
    let f = ok(concat!(
        "module m\n",
        "struct Sst (size: $50) {\n",
        "    id: u16,\n",
        "    art: ArtTile = 0,\n",
        "    sst_custom: [u8; 34] @ $2E,\n",
        "}\n"));
    let Item::Struct(s) = &f.items[0] else { panic!() };
    assert!(s.size.is_some());
    assert_eq!(s.fields.len(), 3);
    assert!(s.fields[1].default.is_some());
    assert!(s.fields[2].offset.is_some());
}

#[test]
fn offsets_decl_parses_members() {
    let f = ok("module m\noffsets Map { Idle: frame_idle, Shoot: frame_shoot }\n");
    let Item::Offsets(o) = &f.items[0] else { panic!() };
    assert_eq!(o.name, "Map");
    assert_eq!(o.members.len(), 2);
    assert_eq!(o.members[0].name, "Idle");
    assert_eq!(o.members[1].name, "Shoot");
}

#[test]
fn vars_region_and_overlay_forms() {
    // region form: `vars upper_ram { ... }`
    let f = ok("module m\nvars upper_ram {\n    Player_Pos_Ring: [u8; 256] @align(256),\n}\n");
    let Item::Vars(v) = &f.items[0] else { panic!() };
    assert_eq!(v.name, None);
    assert_eq!(v.region, vec!["upper_ram".to_string()]);
    assert!(v.fields[0].align.is_some());

    // overlay form: `vars PitcherPlantV: sst_custom { timer: u8 }`
    let f = ok("module m\nvars PitcherPlantV: sst_custom { timer: u8 }\n");
    let Item::Vars(v) = &f.items[0] else { panic!() };
    assert_eq!(v.name.as_deref(), Some("PitcherPlantV"));
    assert_eq!(v.region, vec!["sst_custom".to_string()]);
    assert_eq!(v.fields[0].name, "timer");

    // dotted window form: `vars X: Sst.sst_custom { timer: u8 }`
    let f = ok("module m\nvars X: Sst.sst_custom { timer: u8 }\n");
    let Item::Vars(v) = &f.items[0] else { panic!() };
    assert_eq!(v.region, vec!["Sst".to_string(), "sst_custom".to_string()]);
}

#[test]
fn newtype_plain() {
    let f = ok("module m\nnewtype Frame = u16\npub newtype Angle = u8\n");
    let Item::Newtype(n) = &f.items[0] else { panic!() };
    assert!(!n.public);
    assert_eq!(n.name, "Frame");
    assert!(matches!(&n.underlying, Type::Named(p) if p.segments == vec!["u16"]));
    assert!(n.refine.is_none());
    let Item::Newtype(n) = &f.items[1] else { panic!() };
    assert!(n.public);
}

#[test]
fn newtype_with_where_refinement() {
    let f = ok("module m\nnewtype Percent = u8 where 0..101\n");
    let Item::Newtype(n) = &f.items[0] else { panic!() };
    assert!(matches!(&n.underlying, Type::Named(p) if p.segments == vec!["u8"]));
    let (lo, hi) = n.refine.as_ref().expect("expected a refinement");
    assert!(matches!(lo, Expr::Int(0, _)));
    assert!(matches!(hi, Expr::Int(101, _)));
}

#[test]
fn fixed_point_type() {
    let f = ok("module m\nconst A: fixed<8, 8> = x\n");
    let Item::Const(c) = &f.items[0] else { panic!() };
    assert!(matches!(c.ty, Some(Type::Fixed { i: 8, f: 8 })));
}

#[test]
fn refined_type_in_ordinary_type_position() {
    let f = ok("module m\nconst A: u8 where 0..101 = x\n");
    let Item::Const(c) = &f.items[0] else { panic!() };
    let Some(Type::Refined(base, lo, hi)) = &c.ty else { panic!("{:?}", c.ty) };
    assert!(matches!(**base, Type::Named(_)));
    assert!(matches!(lo, Expr::Int(0, _)));
    assert!(matches!(hi, Expr::Int(101, _)));
}

#[test]
fn fixed_missing_param_is_diagnosed_not_panicking() {
    let (_, diags) = parse_str("module m\nconst A: fixed<> = x\n");
    assert!(!diags.is_empty());
}

#[test]
fn where_with_no_range_is_diagnosed_not_panicking() {
    let (_, diags) = parse_str("module m\nnewtype X = u8 where\n");
    assert!(!diags.is_empty());
}

#[test]
fn comptime_enum_with_payload() {
    let f = ok(concat!(
        "module m\n",
        "comptime enum Token {\n",
        "    Literal(string),\n",
        "    Arg(Width, Operand, TokKind),\n",
        "}\n"));
    let Item::Enum(e) = &f.items[0] else { panic!() };
    assert!(e.comptime);
    assert!(e.repr.is_none());
    assert_eq!(e.variants.len(), 2);
    assert_eq!(e.variants[0].name, "Literal");
    assert_eq!(e.variants[0].payload.len(), 1);
    assert!(matches!(&e.variants[0].payload[0], Type::Named(p) if p.segments == vec!["string"]));
    assert_eq!(e.variants[1].payload.len(), 3);
}

#[test]
fn comptime_enum_may_have_explicit_repr() {
    let f = ok("module m\ncomptime enum Flag: u8 { A, B }\n");
    let Item::Enum(e) = &f.items[0] else { panic!() };
    assert!(e.comptime);
    assert!(matches!(&e.repr, Some(Type::Named(p)) if p.segments == vec!["u8"]));
}

#[test]
fn plain_enum_still_requires_repr() {
    let (f, diags) = parse_str("module m\nenum Anim { Idle }\n");
    assert!(!diags.is_empty());
    let Item::Enum(e) = &f.items[0] else { panic!() };
    assert!(!e.comptime);
}

#[test]
fn data_decls() {
    let f = ok("module m\npub data Def = ObjDef{ code: init }\ndata SineTable: [i8; 256] = deform_sine(amplitude: 20, period: 64)\n");
    let Item::Data(d) = &f.items[0] else { panic!() };
    assert!(d.public && d.ty.is_none());
    let Item::Data(d) = &f.items[1] else { panic!() };
    assert!(matches!(d.ty, Some(Type::Array(_, _))));
    assert!(matches!(d.value, Expr::Call { .. }));
}

// ---- item-position guards (D5.1) ----------------------------------------

#[test]
fn item_level_ensure_parses() {
    let f = ok("module m\nensure(1 == 1, \"ok\")\ndata T: [u8; 1] = [1]\n");
    let Item::Ensure(e) = &f.items[0] else { panic!("expected Item::Ensure, got {:?}", f.items[0]) };
    assert!(!e.fatal);
    let Expr::Call { callee, .. } = &e.call else { panic!("guard call must be an Expr::Call") };
    assert_eq!(callee.segments, vec!["ensure"]);
    assert!(matches!(f.items[1], Item::Data(_)));
}

#[test]
fn item_level_ensure_fatal_parses_in_section() {
    // A guard inside a `section {}` block: it dispatches through the same `item()`
    // path, so it lands in the section's item list with fatal == true.
    let f = ok("module m\nsection s {\nensure_fatal(2 > 1, \"ok\")\n}\n");
    let Item::Section(sec) = &f.items[0] else { panic!("expected Item::Section") };
    let Item::Ensure(e) = &sec.items[0] else { panic!("expected a section-nested guard") };
    assert!(e.fatal);
}

#[test]
fn pub_on_guard_is_diagnosed() {
    let (_f, diags) = parse_str("module m\npub ensure(true, \"x\")\n");
    assert!(
        diags.iter().any(|d| d.message.contains("`pub` is not valid on this declaration")),
        "diags: {diags:?}"
    );
}

#[test]
fn ensure_ident_not_followed_by_paren_still_errors_as_declaration() {
    // The contextual opener only fires on `ensure` immediately followed by `(`.
    let (_f, diags) = parse_str("module m\nensure\n");
    assert!(
        diags.iter().any(|d| d.message.contains("expected a declaration")),
        "diags: {diags:?}"
    );
}

#[test]
fn ensure_usable_as_ordinary_data_name() {
    // Contextual-opener non-regression (D5.1): `ensure` is only a guard when the
    // NEXT token is `(`. As a plain item name (`data ensure`) it is an ordinary
    // identifier, so `data ensure: ... = ...` parses as a normal data item.
    let f = ok("module m\ndata ensure: [u8; 2] = [1, 2]\n");
    let Item::Data(d) = &f.items[0] else { panic!("expected a data item named `ensure`") };
    assert_eq!(d.name, "ensure");
}

#[test]
fn nested_section_is_rejected() {
    // A `section {}` nested inside another `section {}` has no ratified
    // placement-within-placement meaning (locked decision) — and `lower_section_items`
    // has no `Item::Section` arm, so it would silently drop everything inside
    // (data bytes, guards, capacity checks). Reject it loudly at parse time instead.
    let (_f, diags) = parse_str(
        "module m\nsection outer {\n  section inner {\n    data d: [u8; 1] = [$FF]\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.message.contains("[section.nested]")),
        "want [section.nested], got: {diags:?}"
    );
}

// ---- dispatch (D6.B1) ----------------------------------------------------

#[test]
fn dispatch_decl_parses() {
    let f = ok("module m\ndispatch Routines (encoding: word_offsets) { Init: init, Wait: wait }\n");
    let Item::Dispatch(d) = &f.items[0] else { panic!() };
    assert_eq!(d.name, "Routines");
    assert_eq!(d.encoding, DispatchEncoding::WordOffsets);
    assert_eq!(d.members.len(), 2);
    assert_eq!(d.members[0].name, "Init");
}

#[test]
fn dispatch_decl_parses_long_ptrs() {
    let f = ok("module m\ndispatch Routines (encoding: long_ptrs) { Init: init }\n");
    let Item::Dispatch(d) = &f.items[0] else { panic!() };
    assert_eq!(d.encoding, DispatchEncoding::LongPtrs);
}

#[test]
fn pub_dispatch_parses() {
    let f = ok("module m\npub dispatch Routines (encoding: word_offsets) { Init: init }\n");
    let Item::Dispatch(d) = &f.items[0] else { panic!() };
    assert!(d.public);
}

#[test]
fn dispatch_requires_encoding() {
    // No default encoding — research finding R1: enable encodings, impose none.
    let (_f, diags) = parse_str("module m\ndispatch R { A: x }\n");
    assert!(diags.iter().any(|d| d.message.contains("encoding")));
    let (_f, diags) = parse_str("module m\ndispatch R (encoding: sideways) { A: x }\n");
    assert!(diags.iter().any(|d| d.message.contains("word_offsets") && d.message.contains("long_ptrs")));
}

#[test]
fn dispatch_inline_body_parses_as_body_target() {
    // 9a (D9.1): the once-reserved `Member: { … }` form now parses as an
    // inline body (`DispatchTarget::Body`), sugar for an anonymous per-member
    // proc — NOT a diagnostic. Mixing body and label members is legal.
    let (f, diags) = parse_str(
        "module m\ndispatch R (encoding: word_offsets) { A: { rts }, B: b }\n",
    );
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    let Item::Dispatch(d) = &f.items[0] else { panic!() };
    assert_eq!(d.members.len(), 2);
    assert!(matches!(&d.members[0].target, DispatchTarget::Body(_)));
    assert!(matches!(&d.members[1].target, DispatchTarget::Label(_)));
}
