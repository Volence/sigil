//! The blank import `use base._`: pull `base` into the profile's use closure,
//! bind none of its names.
//!
//! THE IDIOM IT SPELLS. A module-level `ensure` is evaluated iff its module is in
//! the closure, and an unreached module gets parse + scan coverage and ZERO body
//! elaboration. For a guard/witness module — one that emits no bytes and that
//! nothing calls into — a `use` edge is the only thing that can put it there, and
//! a selective `use base.{X}` does NOT: a name list injects a clone that
//! re-evaluates in the importing scope and never elaborates the callee. So the
//! edge had to be written as a bare `use base`, which `[import.no-names]` reads as
//! an unfinished import and warns about. `._` is that intent, said out loud.
//!
//! WHAT THESE GATES OWN. The blank form must be the bare form in every respect
//! that reaches the ROM — same closure edge, same elaboration, same bytes — and
//! differ ONLY in the lint. Each half is proven against its own control:
//! elaboration against the module with no `use` at all (which must NOT fire),
//! name-binding against the same tree spelled `use base.{K}` (which must resolve),
//! and lint silence against the bare spelling (which must still warn). A change
//! that silenced `[import.no-names]` everywhere passes the silence half alone.

use std::path::Path;
use std::process::Command;

fn write(dir: &Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

/// Build `entry.emp` under `root`, returning `(success, stderr, bytes)`.
///
/// `bytes` is `None` when the build failed and wrote no output.
fn build(root: &Path) -> (bool, String, Option<Vec<u8>>) {
    let outbin = root.join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("entry.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            outbin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let bytes = std::fs::read(&outbin).ok();
    (out.status.success(), stderr, bytes)
}

/// A guard module in the shape the idiom exists for: emits nothing, exports one
/// name nobody calls, and carries a module-level `ensure` whose failure is the
/// only observable proof that its body elaborated.
const FAILING_GUARD: &str = "module guard\n\
     pub const GK: u16 = 7\n\
     ensure(GK == 8, \"the guard elaborated\")\n";

/// THE CLOSURE EDGE: a module reached ONLY by `use base._` elaborates.
///
/// The control is the same tree with the `use` line deleted. Without it the guard
/// is outside the closure and CANNOT fail, whatever it asserts — so a green from
/// the control and a red from the blank import together say the edge is what did
/// it, and not that the `ensure` fires for everyone.
#[test]
fn blank_import_elaborates_the_module_it_reaches() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "guard.emp", FAILING_GUARD);
    write(root, "entry.emp", "module entry\nuse guard._\npub data D: [u8;1] = [$11]\n");
    let (ok, stderr, _) = build(root);
    assert!(!ok, "the guard behind `use guard._` must have elaborated and failed");
    assert!(
        stderr.contains("the guard elaborated"),
        "the guard's own message must be the failure: {stderr}"
    );

    // Control: no `use` at all — the guard is unreachable and stays silent.
    let tmp2 = tempfile::tempdir().unwrap();
    let root2 = tmp2.path();
    write(root2, "guard.emp", FAILING_GUARD);
    write(root2, "entry.emp", "module entry\npub data D: [u8;1] = [$11]\n");
    let (ok2, stderr2, _) = build(root2);
    assert!(
        ok2 && !stderr2.contains("the guard elaborated"),
        "an unreached guard must not fire, or the test above proves nothing: {stderr2}"
    );
}

/// IT BINDS NOTHING: no `pub` name behind a blank import is in scope.
///
/// BOTH BINDING MECHANISMS, because a `use` reaches names by two separate paths
/// and a gate over one says nothing about the other. A comptime name (a `pub
/// const`) is bound by the ambient injection in `resolve::ambient_from_uses`; a
/// LINK name (a `pub proc`'s label) is bound by the rename map `resolve::imports`
/// builds. Blanking one and leaving the other is a change this catches and a
/// single-name gate does not.
///
/// Each half carries its own control — the same tree spelled `use guard.{Name}`,
/// which must resolve — so a red above means the blank form binds nothing, not
/// that the name was unexportable to begin with.
#[test]
fn blank_import_binds_no_names() {
    const GUARD: &str = "module guard\n\
         pub const GK: u16 = 7\n\
         ensure(GK == 7, \"holds\")\n\
         pub proc GProc () {\n\
             rts\n\
         }\n";
    // (entry body, the name it reaches for, the message an unbound one gives)
    let cases = [
        // Two bytes, not one: the guard's `pub proc` is placed after the entry's
        // section, and an odd-sized entry lands it at an odd address — which the
        // `[layout.odd-item]` gate rejects, failing the control for a reason that
        // has nothing to do with imports.
        ("pub data D: [u8;2] = [GK, GK]\n", "GK", "unknown name `GK`"),
        ("pub proc S () {\n    jmp GProc\n}\n", "GProc", "unresolved name `GProc`"),
    ];

    for (body, name, unbound) in cases {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "guard.emp", GUARD);
        write(root, "entry.emp", &format!("module entry\nuse guard._\n{body}"));
        let (ok, stderr, _) = build(root);
        assert!(!ok, "`{name}` must not be in scope behind a blank import: {stderr}");
        assert!(stderr.contains(unbound), "the failure must be the unbound name: {stderr}");

        let tmp2 = tempfile::tempdir().unwrap();
        let root2 = tmp2.path();
        write(root2, "guard.emp", GUARD);
        write(root2, "entry.emp", &format!("module entry\nuse guard.{{{name}}}\n{body}"));
        let (ok2, stderr2, _) = build(root2);
        assert!(
            ok2,
            "the list spelling must bind `{name}`, or the half above proves nothing: {stderr2}"
        );
    }
}

/// THE LOAD-BEARING CLAIM: `use base._` and `use base` emit the same bytes.
///
/// The two trees are identical but for the `use` line, and the reached module
/// EMITS — so the comparison covers a second module's placement and the entry's
/// own relocation against it, not just a lone byte that could not differ.
#[test]
fn blank_import_is_byte_identical_to_the_bare_form() {
    const REACHED: &str = "module reached\n\
         pub const RK: u16 = $2A\n\
         ensure(RK == $2A, \"holds\")\n\
         pub data R: [u8;4] = [$AA, $BB, $CC, $DD]\n";
    const ENTRY: &str = "module entry\n\
         use reached{SUFFIX}\n\
         pub proc Start () {\n\
             moveq #1, d0\n\
             rts\n\
         }\n\
         pub data D: [u8;3] = [$11, $22, $33]\n";

    let mut built = Vec::new();
    for suffix in ["", "._"] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "reached.emp", REACHED);
        write(root, "entry.emp", &ENTRY.replace("{SUFFIX}", suffix));
        let (ok, stderr, bytes) = build(root);
        assert!(ok, "`use reached{suffix}` must build: {stderr}");
        built.push(bytes.unwrap_or_else(|| panic!("`use reached{suffix}` wrote no output")));
    }
    assert!(!built[0].is_empty(), "the comparison must have bytes to compare");
    assert_eq!(built[0], built[1], "the blank import must be byte-identical to the bare form");
}

/// THE LINT, BOTH DIRECTIONS: silent on `._`, still loud on the bare form.
///
/// The second half is what a blanket silencing would fail. Both spellings are
/// built from the same tree in the same run, so "silent" cannot mean "the lint
/// never ran".
#[test]
fn the_no_names_lint_is_silent_on_blank_and_still_fires_on_bare() {
    const GUARD: &str = "module guard\npub const GK: u16 = 7\nensure(GK == 7, \"holds\")\n";

    let mut seen = Vec::new();
    for suffix in ["", "._"] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "guard.emp", GUARD);
        write(
            root,
            "entry.emp",
            &format!("module entry\nuse guard{suffix}\npub data D: [u8;1] = [$11]\n"),
        );
        let (ok, stderr, _) = build(root);
        assert!(ok, "`use guard{suffix}` must build: {stderr}");
        seen.push(stderr.contains("[import.no-names]"));
    }
    assert!(seen[0], "the bare `use guard` must still warn, a silenced lint is the failure mode");
    assert!(!seen[1], "`use guard._` must not warn");
}

/// The diagnostic TEACHES the blank spelling.
///
/// The lint's whole remaining job is telling an unfinished import from a
/// deliberate closure edge, which it can only do if its message names the form an
/// author is meant to reach for.
#[test]
fn the_no_names_diagnostic_names_the_blank_spelling() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "guard.emp", "module guard\npub const GK: u16 = 7\n");
    write(root, "entry.emp", "module entry\nuse guard\npub data D: [u8;1] = [$11]\n");
    let (ok, stderr, _) = build(root);
    assert!(ok, "a whole-module use is a warning, not an error: {stderr}");
    assert!(
        stderr.contains("use `guard._`") || stderr.contains("`use guard._`"),
        "the message must name the blank spelling for this base: {stderr}"
    );
}

/// `_` has ONE meaning in a module path, and it is the marker.
///
/// The reservation is what keeps the new form from being a second reading of an
/// ordinary identifier. It is enforced at the two places a module path is
/// written, and both are diagnosed BY NAME rather than left to fall out as a
/// resolution failure the author has to decode.
#[test]
fn the_marker_is_never_a_module_path_segment() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "guard.emp", "module guard\npub const GK: u16 = 7\n");

    write(root, "entry.emp", "module entry\nuse guard._.GK\npub data D: [u8;1] = [$11]\n");
    let (ok, stderr, _) = build(root);
    assert!(!ok, "a marker mid-path must be rejected: {stderr}");
    assert!(stderr.contains("blank-import marker"), "diagnosed by name: {stderr}");

    write(root, "_.emp", "module _\npub const K: u16 = 1\n");
    write(root, "entry.emp", "module entry\nuse guard._\npub data D: [u8;1] = [$11]\n");
    let (ok, stderr, _) = build(root);
    assert!(!ok, "a module NAMED `_` must be rejected: {stderr}");
    assert!(stderr.contains("blank-import marker"), "diagnosed by name: {stderr}");
}
