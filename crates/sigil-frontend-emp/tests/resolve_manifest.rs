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
}
