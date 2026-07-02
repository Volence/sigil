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
