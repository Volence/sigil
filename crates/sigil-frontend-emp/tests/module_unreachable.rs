//! `[module.unreachable]` — an `ensure` outside the profile's closure never runs
//! (lens sweep, seat COMPTIME, finding S21).
//!
//! An item-position `ensure` is evaluated IFF its module is lowered, and a module
//! is lowered iff it is in the `use`-reachability closure of the profile's entry.
//! That is a SILENT shape rule: a guard can be written, reviewed and merged while
//! being structurally incapable of firing, and no diagnostic told anyone. Measured
//! on aeon's sonic4 target, 14 modules ship 53 such guards — including
//! `engine.z80_init`, whose `ensure(extern("Z80_IDLE_SIZE") == 40)` is in the
//! registry only for `demo`/`config_b`, so neither shipped sonic4 shape evaluates
//! it or even defers it.
use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{build_program, manifest::Manifest};
use sigil_ir::backend::Cpu;

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

fn opts() -> LowerOptions {
    LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] }
}

/// A guard in an UNREACHED module is reported; the identical guard in a REACHED
/// module is not. Both halves matter: without the second, a lint that fired on
/// everything would also "pass".
#[test]
fn unreached_guards_are_reported_and_reached_ones_are_not() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "entry.emp", "module entry\nuse reached.*\n");
    // Reached: entry `use`s it, so it lowers and its guard really runs.
    write(root, "reached.emp", "module reached\npub const R = 1\nensure(R == 1, \"reached\")\n");
    // Orphan: nothing `use`s it. Its guard cannot fail, whatever it asserts —
    // note this one is FALSE, and the build is still clean.
    write(root, "orphan.emp", "module orphan\npub const O = 1\nensure(O == 2, \"never evaluated\")\n");

    let (manifest, _d) = Manifest::scan(root);
    let (_s, _a, diags) = build_program(&manifest, "entry", None, &opts());

    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "an unreached FALSE guard must not fail the build — that is the defect: {diags:?}"
    );
    let hits: Vec<&str> = diags
        .iter()
        .filter(|d| d.message.contains("[module.unreachable]"))
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(hits.len(), 1, "exactly the orphan should be reported, got {hits:?}");
    assert!(hits[0].contains("orphan"), "{hits:?}");
    assert!(hits[0].contains('1'), "the count of unevaluated guards is the point: {hits:?}");
}

/// An unreached module with NO guards is silent — the lint reports unevaluated
/// GUARDS, not unreachability, so it does not become background noise that gets
/// filtered out and stops being read.
#[test]
fn unreached_module_without_guards_is_silent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "entry.emp", "module entry\n");
    write(root, "orphan.emp", "module orphan\npub const O = 1\n");

    let (manifest, _d) = Manifest::scan(root);
    let (_s, _a, diags) = build_program(&manifest, "entry", None, &opts());
    assert!(
        !diags.iter().any(|d| d.message.contains("[module.unreachable]")),
        "a guard-free orphan must be silent: {diags:?}"
    );
}
