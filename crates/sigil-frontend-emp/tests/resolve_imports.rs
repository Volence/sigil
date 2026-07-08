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
