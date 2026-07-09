//! `--deny-todo` (S2-D11(e)): `sigil emp` builds a module carrying `todo!`
//! holes with exit 0 by default (each hole named via `[todo.present]`,
//! warning tier); `--deny-todo` promotes those to errors for release builds.
//!
//! Spawn-the-binary shape mirrors `end_to_end.rs`.

use std::process::Command;

fn unique_temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sigil_deny_todo_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const SRC_WITH_TODO: &str = "\
module m
proc p () {
    nop
    todo!(\"finish the brain\")
}
";

#[test]
fn todo_builds_with_warning_by_default() {
    let dir = unique_temp_dir();
    let emp = dir.join("m.emp");
    std::fs::write(&emp, SRC_WITH_TODO).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg("emp")
        .arg(&emp)
        .output()
        .expect("spawn sigil");

    assert!(
        out.status.success(),
        "todo! must not fail the build by default; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("[todo.present]") && stderr.contains("finish the brain"),
        "the hole is still named on stderr: {stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deny_todo_promotes_to_error() {
    let dir = unique_temp_dir();
    let emp = dir.join("m.emp");
    std::fs::write(&emp, SRC_WITH_TODO).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg("emp")
        .arg(&emp)
        .arg("--deny-todo")
        .output()
        .expect("spawn sigil");

    assert!(
        !out.status.success(),
        "--deny-todo must fail a build carrying todo!; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("[todo.present]"), "the promoted hole is named: {stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deny_todo_leaves_unreachable_alone() {
    let dir = unique_temp_dir();
    let emp = dir.join("m.emp");
    std::fs::write(
        &emp,
        "module m\nproc p () {\n    unreachable!\n}\n",
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg("emp")
        .arg(&emp)
        .arg("--deny-todo")
        .output()
        .expect("spawn sigil");

    assert!(
        out.status.success(),
        "unreachable! is a permanent trap, not a hole — --deny-todo ignores it; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
