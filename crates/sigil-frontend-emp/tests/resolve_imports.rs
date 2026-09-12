use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::imports::{
    canonical, use_decl_errors, use_decls, ExportIndex, ResolveEnv,
};

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

#[test]
fn duplicate_module_in_index_does_not_look_ambiguous() {
    // Same module passed twice must NOT double-count its exports, or `suggest_use`
    // would wrongly treat the name as exported by two modules.
    let (helpers, _) = parse_str("module engine.helpers\npub proc Draw_Sprite (a0: *u8) {}\n");
    let (obj, _) = parse_str("module badniks.plant\nproc init (a0: *u8) {}\n");
    let idx = ExportIndex::build(&[
        ("engine.helpers", &helpers),
        ("engine.helpers", &helpers), // duplicated on purpose
        ("badniks.plant", &obj),
    ]);
    let (env, _) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert_eq!(
        env.suggest_use("Draw_Sprite"),
        Some("add `use engine.helpers.{Draw_Sprite}`".to_string())
    );
}

#[test]
fn glob_imports_all_exports() {
    let (helpers, _) =
        parse_str("module engine.helpers\npub proc Draw_Sprite (a0: *u8) {}\npub proc Hide (a0: *u8) {}\n");
    let (obj, _) = parse_str("module badniks.plant\nuse engine.helpers.*\nproc init (a0: *u8) {}\n");
    let idx = ExportIndex::build(&[("engine.helpers", &helpers), ("badniks.plant", &obj)]);
    let (env, diags) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error), "{diags:?}");
    assert_eq!(env.resolve("Draw_Sprite"), Some("engine.helpers.Draw_Sprite".to_string()));
    assert_eq!(env.resolve("Hide"), Some("engine.helpers.Hide".to_string()));
}

#[test]
fn glob_on_unknown_base_errors() {
    let (obj, _) = parse_str("module badniks.plant\nuse bogus.*\nproc init (a0: *u8) {}\n");
    let idx = ExportIndex::build(&[("badniks.plant", &obj)]);
    let (_env, diags) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "expected an error for a glob whose base matches no module, got {diags:?}"
    );
}

#[test]
fn use_list_of_nonexistent_pub_name_errors() {
    let (helpers, _) = parse_str("module engine.helpers\npub proc Draw_Sprite (a0: *u8) {}\n");
    let (obj, _) = parse_str("module badniks.plant\nuse engine.helpers.{Nope}\nproc init (a0: *u8) {}\n");
    let idx = ExportIndex::build(&[("engine.helpers", &helpers), ("badniks.plant", &obj)]);
    let (_env, diags) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "expected `no pub name` error, got {diags:?}"
    );
}

#[test]
fn suggest_use_is_none_when_two_modules_export_the_name() {
    // Two modules export `Foo` → the fix-it is ambiguous, so `suggest_use` = None.
    let (a, _) = parse_str("module mod.a\npub proc Foo (a0: *u8) {}\n");
    let (b, _) = parse_str("module mod.b\npub proc Foo (a0: *u8) {}\n");
    let (obj, _) = parse_str("module badniks.plant\nproc init (a0: *u8) {}\n");
    let idx = ExportIndex::build(&[("mod.a", &a), ("mod.b", &b), ("badniks.plant", &obj)]);
    let (env, _) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert_eq!(env.suggest_use("Foo"), None);
}

#[test]
fn pub_equ_import_binds_the_bare_link_name() {
    // `pub equ` is a module-visible item like every other `pub` declaration, so
    // a `use` of it resolves. Unlike the others it binds to the BARE name: a
    // `pub equ` mints one plain, program-unique link symbol (that is the
    // construct's cross-seam purpose), and its definition is never renamed to a
    // module-qualified canonical, so an import must not be either.
    let (a, _) = parse_str("module mod.a\npub equ WIDTH = $20\n");
    let (obj, _) = parse_str("module badniks.plant\nuse mod.a.{WIDTH}\nproc init (a0: *u8) {}\n");
    let idx = ExportIndex::build(&[("mod.a", &a), ("badniks.plant", &obj)]);
    let (env, diags) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "importing a `pub equ` must not error, got {diags:?}"
    );
    assert_eq!(env.resolve("WIDTH"), Some("WIDTH".to_string()));
    assert_ne!(env.resolve("WIDTH"), Some(canonical("mod.a", "WIDTH")));
}

#[test]
fn private_equ_is_not_exported() {
    // Control arm for `pub_equ_import_binds_the_bare_link_name`: without `pub`
    // the equ stays module-private, so the export index does not carry it and
    // the `use` errors. A change that exported every `equ` passes the arm above
    // and fails only here.
    let (a, _) = parse_str("module mod.a\nequ WIDTH = $20\n");
    let (obj, _) = parse_str("module badniks.plant\nuse mod.a.{WIDTH}\nproc init (a0: *u8) {}\n");
    let idx = ExportIndex::build(&[("mod.a", &a), ("badniks.plant", &obj)]);
    assert!(!idx.is_exported("mod.a", "WIDTH"));
    let (env, diags) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message == "module `mod.a` has no `pub` name `WIDTH`"),
        "expected the module-visibility rejection, got {diags:?}"
    );
    assert_eq!(env.resolve("WIDTH"), None);
}

/// The import rule applied without binding (`use_decl_errors`, what a path that
/// removes or never resolves a `use` calls): a listed name the module does not
/// export is refused, private or absent alike, each at the `use`'s own span, and a
/// real `pub` name beside them is not.
#[test]
fn the_import_rule_refuses_each_listed_name_the_module_does_not_export() {
    let (a, _) = parse_str("module mod.a\npub const W = 4\nconst PRIV = 2\n");
    let (b, _) = parse_str("module mod.b\nuse mod.a.{W, NOPE, PRIV}\n");
    let idx = ExportIndex::build(&[("mod.a", &a), ("mod.b", &b)]);
    let uses = use_decls(&b.items);
    assert_eq!(uses.len(), 1, "{uses:?}");
    let errors = use_decl_errors(uses[0], &idx);
    for d in &errors {
        assert_eq!(d.level, sigil_span::Level::Error);
        assert_eq!(d.primary, uses[0].span, "anchored at the `use`, not at a reader");
    }
    let messages: Vec<&str> = errors.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        messages,
        vec!["module `mod.a` has no `pub` name `NOPE`", "module `mod.a` has no `pub` name `PRIV`"]
    );
}

/// The resolve pass and the unbound rule speak with one voice: the same `use` yields
/// the same Errors from both, so a message seen on a standalone path is the map
/// build's message.
#[test]
fn the_import_rule_and_the_resolve_pass_refuse_identically() {
    let (a, _) = parse_str("module mod.a\npub const W = 4\n");
    let (b, _) = parse_str("module mod.b\nuse mod.a.{NOPE}\nuse mod.a.{W}\n");
    let idx = ExportIndex::build(&[("mod.a", &a), ("mod.b", &b)]);
    let (_env, resolved) = ResolveEnv::build("mod.b", &b, &idx, None);
    let unbound: Vec<_> = use_decls(&b.items).into_iter().flat_map(|u| use_decl_errors(u, &idx)).collect();
    assert_eq!(unbound.len(), 1, "{unbound:?}");
    assert_eq!(resolved, unbound);
}

/// A glob over a module that exports nothing is refused; a glob over one that
/// exports something, and the two forms that name nothing, have nothing to check.
#[test]
fn the_import_rule_refuses_only_a_glob_that_brings_nothing() {
    let (a, _) = parse_str("module mod.a\npub const W = 4\n");
    let (e, _) = parse_str("module mod.empty\nconst K = 1\n");
    let (b, _) = parse_str("module mod.b\nuse mod.empty.*\nuse mod.a.*\nuse mod.a\nuse mod.a._\n");
    let idx = ExportIndex::build(&[("mod.a", &a), ("mod.empty", &e), ("mod.b", &b)]);
    let per_use: Vec<Vec<String>> = use_decls(&b.items)
        .into_iter()
        .map(|u| use_decl_errors(u, &idx).into_iter().map(|d| d.message).collect())
        .collect();
    assert_eq!(
        per_use,
        vec![
            vec!["glob `use mod.empty.*` matches no module with `pub` names".to_string()],
            vec![],
            vec![],
            vec![],
        ]
    );
}

/// `has_module` tells a module that exports nothing from one that does not exist,
/// the distinction a standalone path needs before it can say which is wrong.
#[test]
fn has_module_tells_an_empty_module_from_a_missing_one() {
    let (e, _) = parse_str("module mod.empty\nconst K = 1\n");
    let idx = ExportIndex::build(&[("mod.empty", &e)]);
    assert!(idx.has_module("mod.empty"));
    assert!(!idx.has_module("mod.missing"));
}
