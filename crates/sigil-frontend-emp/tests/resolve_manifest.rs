use sigil_frontend_emp::resolve::manifest::Manifest;

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

#[test]
fn indexes_modules_by_header_and_lints_path_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "badniks/pitcher_plant.emp", "module badniks.pitcher_plant\n");
    write(root, "engine/helpers.emp", "module engine.helpers\n");
    // Header says one thing, directory says another → LINT, not error.
    write(root, "misplaced/here.emp", "module engine.objects.sst\n");

    let (manifest, diags) = Manifest::scan(root);
    assert!(manifest.by_id.contains_key("badniks.pitcher_plant"));
    assert!(manifest.by_id.contains_key("engine.helpers"));
    assert!(manifest.by_id.contains_key("engine.objects.sst"));
    // The mismatch is a warning, and NOTHING is an error.
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error));
    assert!(diags.iter().any(|d| d.level == sigil_span::Level::Warning
        && d.message.contains("engine.objects.sst")));
    // The lint must NOT over-fire: the two well-placed modules trigger no
    // warning, so exactly one Warning is emitted in total.
    let warnings: Vec<_> =
        diags.iter().filter(|d| d.level == sigil_span::Level::Warning).collect();
    assert_eq!(warnings.len(), 1, "expected exactly one warning, got {warnings:?}");
    assert!(warnings.iter().all(|d| !d.message.contains("badniks.pitcher_plant")
        && !d.message.contains("engine.helpers")));

    // Per-file SourceId attribution: each module's header span points at a
    // distinct source, and `sources` resolves that id back to the file path.
    let ids: std::collections::HashSet<_> =
        manifest.modules.iter().map(|m| m.file.module.span.source).collect();
    assert_eq!(ids.len(), manifest.modules.len(), "each module needs a distinct SourceId");
    for m in &manifest.modules {
        assert_eq!(manifest.sources.get(&m.file.module.span.source), Some(&m.path));
    }
}

#[test]
fn duplicate_module_id_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "a/dup.emp", "module shared.thing\n");
    write(root, "b/dup.emp", "module shared.thing\n");

    let (manifest, diags) = Manifest::scan(root);
    assert_eq!(manifest.modules.len(), 2);
    assert!(diags.iter().any(|d| d.level == sigil_span::Level::Error
        && d.message.contains("shared.thing")));
    // Last-wins: `by_id` points at the final occurrence.
    assert_eq!(manifest.by_id.get("shared.thing"), Some(&1));
}

#[test]
fn nonexistent_root_reports_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("does_not_exist");

    let (manifest, diags) = Manifest::scan(&missing);
    assert!(manifest.modules.is_empty());
    assert!(diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "expected a root-read-failure error, got {diags:?}");
}
