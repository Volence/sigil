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
    // The happy path emits a deterministic image: one `rts` (Draw_Sprite) + one
    // `jmp Draw_Sprite`. Pin the exact length so a mis-linked cross-module fixup
    // (which would change the emitted width/bytes) can't pass silently.
    assert!(out.exists());
    assert_eq!(std::fs::metadata(&out).unwrap().len(), 4, "expected a 4-byte image");
}

#[test]
fn transitive_chain_discovers_third_module() {
    // A `use`s B, B `use`s C. A branches to a name imported from B; B branches to
    // a name imported from C. C is only reachable THROUGH B — this proves the
    // `reachable_modules` BFS discovers it transitively (two 2-module tests never
    // exercise transitivity).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "chain/c.emp", "module chain.c\npub proc c_fn (a0: *u8) {\n    rts\n}\n");
    write(
        root,
        "chain/b.emp",
        "module chain.b\nuse chain.c.{c_fn}\npub proc b_fn (a0: *u8) {\n    jmp c_fn\n}\n",
    );
    write(
        root,
        "chain/a.emp",
        "module chain.a\nuse chain.b.{b_fn}\nproc init (a0: *u8) {\n    jmp b_fn\n}\n",
    );
    let out = root.join("out.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("chain/a.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "transitive 3-module compile should succeed");
    assert!(out.exists() && std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn unknown_module_id_reports_diagnostic() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nuse missing.mod.{Foo}\nproc init (a0: *u8) {\n    rts\n}\n",
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
        stderr.contains("no module `missing.mod` found under the scan root"),
        "stderr was: {stderr}"
    );
}

#[test]
fn prelude_types_resolve_without_use() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // Prelude exports a struct type used by the object module with NO `use`.
    write(root, "prelude.emp", "module prelude\npub struct ObjDef (size: 4) { code: *u8 }\n");
    write(
        root,
        "badniks/plant.emp",
        "module badniks.plant\nproc init (a0: *u8) {\n    rts\n}\n\
         pub data Def = ObjDef{ code: \"init\" }\n",
    );
    let out = root.join("out.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            root.join("badniks/plant.emp").to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--prelude",
            "prelude",
            "-o",
            out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "prelude struct should resolve without an explicit use");
    assert!(std::fs::metadata(&out).unwrap().len() >= 4); // Def = one *u8 pointer (fixup to init)
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
