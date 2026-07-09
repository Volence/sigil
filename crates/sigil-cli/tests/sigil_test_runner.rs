//! `sigil test` (S2-D11(a)): the comptime-test runner CLI — exit 0 on
//! all-pass, exit 1 on any failure, one `test <module>::<name> ... ok|FAILED`
//! line per test. Spawn shape mirrors `end_to_end.rs`.

use std::process::Command;

fn unique_temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sigil_test_runner_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn all_passing_exits_zero() {
    let dir = unique_temp_dir();
    let emp = dir.join("m.emp");
    std::fs::write(
        &emp,
        "module m\n\
         comptime fn sq(x: int) -> int {\n    return x * x\n}\n\
         comptime test \"squares\" {\n    ensure(sq(3) == 9, \"3^2\")\n}\n\
         comptime test \"rejects\" (expect_error: \"[struct.missing-field]\") {\n\
             let bad = S{ }\n}\n\
         struct S { a: u8 }\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg("test")
        .arg(&emp)
        .output()
        .expect("spawn sigil");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "stdout: {stdout}\nstderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("test m::squares ... ok"), "{stdout}");
    assert!(stdout.contains("test m::rejects ... ok"), "{stdout}");
    assert!(stdout.contains("2 passed; 0 failed"), "{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failing_test_exits_nonzero_and_names_itself() {
    let dir = unique_temp_dir();
    let emp = dir.join("m.emp");
    std::fs::write(
        &emp,
        "module m\n\
         comptime test \"broken\" {\n    ensure(1 == 2, \"math failed: {1}\")\n}\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg("test")
        .arg(&emp)
        .output()
        .expect("spawn sigil");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "must exit nonzero: {stdout}");
    assert!(stdout.contains("test m::broken ... FAILED"), "{stdout}");
    assert!(stdout.contains("math failed: 1"), "the guard message surfaces: {stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn root_sweeps_every_module() {
    let dir = unique_temp_dir();
    std::fs::write(
        dir.join("a.emp"),
        "module a\ncomptime test \"in a\" {\n    ensure(true, \"x\")\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.emp"),
        "module b\ncomptime test \"in b\" {\n    ensure(true, \"x\")\n}\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg("test")
        .arg("--root")
        .arg(&dir)
        .output()
        .expect("spawn sigil");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("test a::in a ... ok"), "{stdout}");
    assert!(stdout.contains("test b::in b ... ok"), "{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}
