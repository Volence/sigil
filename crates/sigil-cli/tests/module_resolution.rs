use std::process::Command;

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

#[test]
fn two_modules_cross_reference_and_link() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "engine/helpers.emp",
        "module engine.helpers\npub proc Draw_Sprite (a0: *u8) {\n    rts\n}\n",
    );
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nuse engine.helpers.{Draw_Sprite}\nproc init (a0: *u8) {\n    jmp Draw_Sprite\n}\n",
    );
    let out = root.join("out.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("badniks/plant.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "multi-module compile should succeed");
    assert!(out.exists() && std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn missing_use_reports_add_use_fixit() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "engine/helpers.emp",
        "module engine.helpers\npub proc Draw_Sprite (a0: *u8) {\n    rts\n}\n",
    );
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nproc init (a0: *u8) {\n    jmp Draw_Sprite\n}\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("badniks/plant.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("add `use engine.helpers.{Draw_Sprite}`"),
        "stderr was: {stderr}"
    );
}
