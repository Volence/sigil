//! Final review M1: A4's hidden copies (`$def$name`) never reach an A1
//! `[const.out-of-range]` message, and the refusal prints once.
//!
//! `const Y: u8 = 300` in `engine.helper`, read by `pub comptime fn f()` there and
//! called from `app.main`. A4 injects a hidden copy `$engine.helper$Y` into
//! main's composed file so `f` reads helper's `Y`; the const range check ran on
//! that copy too, so the build printed the refusal twice, once "in 'Y'" and once
//! "in '$engine.helper$Y'", a name source cannot spell. The defining module is in
//! the `use` closure and checks the original, so the copy is skipped.

use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{build_program, manifest::Manifest};
use sigil_ir::backend::Cpu;
use sigil_span::Level;

#[test]
fn a_hidden_copy_does_not_repeat_the_const_refusal_under_its_hidden_name() {
    let dir = tempfile::tempdir().unwrap();
    let w = |rel: &str, src: &str| {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, src).unwrap();
    };
    w(
        "engine/helper.emp",
        "module engine.helper\nconst Y: u8 = 300\npub comptime fn f() -> Code {\n    return asm { moveq #Y, d0 }\n}\n",
    );
    w("app/main.emp", "module app.main\nuse engine.helper.{f}\npub proc main () {\n    f()\n    rts\n}\n");
    let (manifest, _) = Manifest::scan(dir.path());
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    let (_sections, _asserts, diags) = build_program(&manifest, "app.main", None, &opts);
    let errs: Vec<&str> =
        diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.as_str()).collect();
    let consts: Vec<&&str> = errs.iter().filter(|m| m.contains("[const.out-of-range]")).collect();
    assert_eq!(
        consts,
        [&"[const.out-of-range] 300 does not fit u8 (0..=255), in 'Y'"],
        "all errors: {errs:?}"
    );
    assert!(errs.iter().all(|m| !m.contains("$engine.helper$")), "a hidden name leaked: {errs:?}");
}
