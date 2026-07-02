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
fn enum_decl() {
    let f = ok("module m\nenum Anim: u8 { Idle = 0, Seed = 1, Shoot = 2 }\n");
    let Item::Enum(e) = &f.items[0] else { panic!() };
    assert_eq!(e.name, "Anim");
    assert_eq!(e.variants.len(), 3);
    assert_eq!(e.variants[2].0, "Shoot");
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
fn vars_region_and_overlay_forms() {
    // region form: `vars upper_ram { ... }`
    let f = ok("module m\nvars upper_ram {\n    Player_Pos_Ring: [u8; 256] @align(256),\n}\n");
    let Item::Vars(v) = &f.items[0] else { panic!() };
    assert_eq!(v.name, None);
    assert_eq!(v.region, "upper_ram");
    assert!(v.fields[0].align.is_some());

    // overlay form: `vars PitcherPlantV: sst_custom { timer: u8 }`
    let f = ok("module m\nvars PitcherPlantV: sst_custom { timer: u8 }\n");
    let Item::Vars(v) = &f.items[0] else { panic!() };
    assert_eq!(v.name.as_deref(), Some("PitcherPlantV"));
    assert_eq!(v.region, "sst_custom");
    assert_eq!(v.fields[0].name, "timer");
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
