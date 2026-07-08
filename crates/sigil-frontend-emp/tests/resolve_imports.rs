use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::imports::{canonical, ExportIndex, ResolveEnv};

#[test]
fn canonical_is_module_qualified() {
    assert_eq!(canonical("badniks.pitcher_plant", "init"), "badniks.pitcher_plant.init");
}

#[test]
fn use_list_resolves_to_defining_module_canonical() {
    let (helpers, _) = parse_str("module engine.helpers\npub proc Draw_Sprite (a0: *u8) {}\n");
    let (obj, _) = parse_str(
        "module badniks.plant\nuse engine.helpers.{Draw_Sprite}\nproc init (a0: *u8) {}\n",
    );
    let idx = ExportIndex::build(&[("engine.helpers", &helpers), ("badniks.plant", &obj)]);
    let (env, diags) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert!(diags.is_empty());
    // Own private proc → own canonical.
    assert_eq!(env.resolve("init"), Some("badniks.plant.init".to_string()));
    // Imported name → defining module's canonical.
    assert_eq!(env.resolve("Draw_Sprite"), Some("engine.helpers.Draw_Sprite".to_string()));
}

#[test]
fn unimported_but_exported_elsewhere_yields_add_use_fixit() {
    let (helpers, _) = parse_str("module engine.helpers\npub proc Draw_Sprite (a0: *u8) {}\n");
    let (obj, _) = parse_str("module badniks.plant\nproc init (a0: *u8) {}\n"); // NO use
    let idx = ExportIndex::build(&[("engine.helpers", &helpers), ("badniks.plant", &obj)]);
    let (env, _) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    // Not directly resolvable, but the env can SUGGEST the missing use.
    assert_eq!(env.resolve("Draw_Sprite"), None);
    assert_eq!(
        env.suggest_use("Draw_Sprite"),
        Some("add `use engine.helpers.{Draw_Sprite}`".to_string())
    );
}

#[test]
fn use_shadows_prelude_silently() {
    // The prelude exports `Foo`, and the module `use`s a DIFFERENT module's `Foo`.
    // The `use` must win over the prelude, with NO collision error.
    let (prelude, _) = parse_str("module std.prelude\npub proc Foo (a0: *u8) {}\n");
    let (other, _) = parse_str("module other\npub proc Foo (a0: *u8) {}\n");
    let (obj, _) = parse_str("module badniks.plant\nuse other.{Foo}\nproc init (a0: *u8) {}\n");
    let idx = ExportIndex::build(&[
        ("std.prelude", &prelude),
        ("other", &other),
        ("badniks.plant", &obj),
    ]);
    let (env, diags) =
        ResolveEnv::build("badniks.plant", &obj, &idx, Some(("std.prelude", &prelude)));
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "{diags:?}");
    assert_eq!(env.resolve("Foo"), Some("other.Foo".to_string()));
}

#[test]
fn use_vs_use_conflict_still_errors() {
    // Two different `use` imports bringing the same short name `Foo` from two
    // different modules is a genuine equal-precedence collision → error.
    let (a, _) = parse_str("module mod.a\npub proc Foo (a0: *u8) {}\n");
    let (b, _) = parse_str("module mod.b\npub proc Foo (a0: *u8) {}\n");
    let (obj, _) = parse_str(
        "module badniks.plant\nuse mod.a.{Foo}\nuse mod.b.{Foo}\nproc init (a0: *u8) {}\n",
    );
    let idx =
        ExportIndex::build(&[("mod.a", &a), ("mod.b", &b), ("badniks.plant", &obj)]);
    let (_env, diags) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "expected a use-vs-use collision error, got {diags:?}"
    );
}
