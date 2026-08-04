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

// ── t24 control: the scan must not descend into a NESTED CHECKOUT ──────────────
// A `git worktree` (or nested clone) under the scan root is a byte-identical COPY
// of the same `.emp` tree; walking into it reports every module `declared twice`
// (the Stage-2 flip bug: `build.sh` drives `sigil build` from the main checkout,
// which routinely contains `.worktrees/`). Two nested-checkout signatures must be
// ignored — a `.worktrees/` container and a subdir carrying its own `.git` — while
// the scan ROOT (legitimately a worktree with a `.git` FILE) is still scanned.
#[test]
fn scan_ignores_nested_checkouts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // The scan root is ITSELF a worktree: a `.git` FILE at the root (the gitdir
    // pointer a `git worktree` plants). The root's own module MUST still be found.
    std::fs::write(root.join(".git"), "gitdir: /somewhere/.git/worktrees/x\n").unwrap();
    write(root, "engine/thing.emp", "module engine.thing\n");

    // Signature (1): a `.worktrees/<branch>/` copy of the same module.
    write(root, ".worktrees/flip/engine/thing.emp", "module engine.thing\n");

    // Signature (2): a nested checkout carrying its OWN `.git` (worktree pointer),
    // NOT under `.worktrees/` — the general nested-repo signature.
    write(root, "nested_clone/engine/thing.emp", "module engine.thing\n");
    std::fs::write(root.join("nested_clone/.git"), "gitdir: /elsewhere\n").unwrap();

    let (manifest, diags) = Manifest::scan(root);

    // The decoys are ignored: `engine.thing` is found EXACTLY ONCE, no dup error.
    assert!(manifest.by_id.contains_key("engine.thing"), "root module must be scanned");
    let count = manifest.modules.iter().filter(|m| m.id == "engine.thing").count();
    assert_eq!(count, 1, "nested-checkout copies must be ignored, found {count} `engine.thing`");
    assert!(
        !diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "no `declared twice` (or any) error expected; got {:?}",
        diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
}

/// `SourceIndex` is the ONE location authority both diagnostic tiers render
/// through, so it must turn a real span into `path:line:col` and must answer
/// `None` — never a fabricated position — for a source it cannot read.
#[test]
fn source_index_locates_real_spans_and_declines_unreadable_ones() {
    use sigil_frontend_emp::resolve::manifest::SourceIndex;
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "engine/thing.emp", "module engine.thing\n\nproc P () {\n}\n");

    let (mut manifest, _) = Manifest::scan(root);
    let real = manifest.modules[0].file.module.span.source;

    // A synthetic module registered at a path that does not exist — exactly the
    // shape the native driver's generated entry takes.
    let ghost = sigil_span::SourceId(manifest.modules.len() as u32);
    manifest.sources.insert(ghost, root.join("__generated__.emp"));

    let index = SourceIndex::new(&manifest);
    let at = |source, start| sigil_span::Span { source, start, end: start };

    // Offset 21 is the first byte of line 3 ("module engine.thing\n" = 20, "\n" = 1).
    let loc = index.locate(at(real, 21)).expect("a readable source must locate");
    assert!(loc.ends_with("engine/thing.emp:3:1"), "got {loc}");
    assert_eq!(index.locate(at(real, 0)).unwrap().split(':').next_back(), Some("1"));

    assert_eq!(index.locate(at(ghost, 0)), None, "an unreadable source has no position");
    assert_eq!(
        index.locate(at(sigil_span::SourceId(99), 0)),
        None,
        "an out-of-range source id must decline, not panic"
    );
}

/// The path lint carries the corpus's `[area.name]` id convention, so it tallies
/// as a named class in the build's warn-tier summary instead of `unclassified`.
#[test]
fn path_mismatch_lint_carries_its_id() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "misplaced/here.emp", "module engine.objects.sst\n");
    let (_, diags) = Manifest::scan(root);
    assert!(
        diags.iter().any(|d| d.message.starts_with("[module.path-mismatch]")),
        "got {diags:?}"
    );
}
