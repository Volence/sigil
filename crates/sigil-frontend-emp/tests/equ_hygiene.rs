//! C1 item 4 — equ hygiene: a non-`pub` equ is owner-mangled (`$module$NAME`)
//! by the rename pass, so two modules can each declare a same-named private equ
//! without colliding in the flat link table; a `pub equ` keeps its plain
//! cross-seam name, and two `pub equ`s of the same name are a loud
//! `[equ.collision]` naming BOTH modules. The mangling rewrites the equ's own
//! symbol AND its in-module references in lockstep (an equ used as an absolute
//! operand address is a symbol reference, not an inlined value).

use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{build_program, manifest::Manifest, place_sequential};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

/// Build a program from `(rel, src)` files, entering at `entry`. Returns the
/// concatenated (post-rename) sections and the build diagnostics.
fn build(files: &[(&str, &str)], entry: &str) -> (Vec<Section>, Vec<Diagnostic>) {
    let dir = tempfile::tempdir().unwrap();
    for (rel, content) in files {
        write(dir.path(), rel, content);
    }
    let (manifest, mdiags) = Manifest::scan(dir.path());
    assert!(mdiags.iter().all(|d| d.level != Level::Error), "manifest errors: {mdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let (sections, _asserts, diags) = build_program(&manifest, entry, None, &opts);
    (sections, diags)
}

fn errors(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.as_str()).collect()
}

fn equ_names(sections: &[Section]) -> Vec<String> {
    sections.iter().flat_map(|s| s.equ_syms.iter().map(|e| e.name.clone())).collect()
}

/// Two modules each declaring a same-named PRIVATE `equ SLOT` — used as an
/// absolute operand (a symbol reference) — link cleanly: each is mangled to its
/// own `$module$SLOT`, so there is no flat-table collision and the reference
/// resolves to the same-module definition.
#[test]
fn two_modules_same_private_equ_do_not_collide() {
    let helper = "\
module engine.helper
equ SLOT = $10
pub proc helper_entry (a0: *u8) {
    move.w SLOT, d0
    rts
}
";
    let cons = "\
module app.main
use engine.helper.{helper_entry}
equ SLOT = $20
pub proc main (a0: *u8) {
    move.w SLOT, d0
    jsr helper_entry
    rts
}
";
    let (mut sections, diags) =
        build(&[("engine/helper.emp", helper), ("app/main.emp", cons)], "app.main");
    assert!(errors(&diags).is_empty(), "unexpected build errors: {:?}", errors(&diags));

    // Both private equs are mangled to distinct owner-scoped symbols.
    let names = equ_names(&sections);
    assert!(
        names.iter().any(|n| n == "$engine.helper$SLOT"),
        "helper's private equ must be mangled to $engine.helper$SLOT, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "$app.main$SLOT"),
        "main's private equ must be mangled to $app.main$SLOT, got: {names:?}"
    );

    // And the whole program places + links without a duplicate-symbol error —
    // the collision the mangling prevents.
    place_sequential(&mut sections, 0x0000);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed (private equs must not collide): {d:?}"));
}

/// A `pub equ` keeps its plain cross-seam name (it is the contract surface) — it
/// is NOT mangled.
#[test]
fn pub_equ_keeps_plain_name() {
    let src = "\
module app.only
pub equ PUBLIC_SLOT = $30
equ PRIVATE_SLOT = $40
pub proc p (a0: *u8) {
    move.w PUBLIC_SLOT, d0
    move.w PRIVATE_SLOT, d1
    rts
}
";
    let (sections, diags) = build(&[("app/only.emp", src)], "app.only");
    assert!(errors(&diags).is_empty(), "unexpected build errors: {:?}", errors(&diags));
    let names = equ_names(&sections);
    assert!(names.iter().any(|n| n == "PUBLIC_SLOT"), "pub equ keeps its plain name: {names:?}");
    assert!(
        names.iter().any(|n| n == "$app.only$PRIVATE_SLOT"),
        "private equ is mangled: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n == "PRIVATE_SLOT"),
        "the plain private name must NOT survive: {names:?}"
    );
}

/// Two modules declaring the SAME `pub equ` collide — a loud `[equ.collision]`
/// that says "equ" and NAMES BOTH modules.
#[test]
fn two_pub_equs_same_name_collide_naming_both_modules() {
    let a = "\
module app.a
pub equ SHARED = $1
";
    let b = "\
module app.b
use app.a.{SHARED}
pub equ SHARED = $2
pub proc go (a0: *u8) { rts }
";
    // Enter at b (which `use`s a, making both reachable).
    let (_sections, diags) = build(&[("app/a.emp", a), ("app/b.emp", b)], "app.b");
    let errs = errors(&diags);
    let hit = errs
        .iter()
        .find(|e| e.contains("[equ.collision]"))
        .unwrap_or_else(|| panic!("expected [equ.collision], got: {errs:?}"));
    assert!(hit.contains("SHARED"), "collision names the equ: {hit}");
    assert!(hit.contains("app.a") && hit.contains("app.b"), "collision names both modules: {hit}");
    assert!(hit.contains("equ"), "collision says 'equ': {hit}");
}
