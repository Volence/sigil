//! TRANCHE 0 acceptance (the kickoff brief's gate, 2026-07-09): the D2.30
//! preview exhibit `examples/previews/pitcher_plant_script_next.emp` compiles
//! with ZERO diagnostics once its ONE `code_word` header line is patched to a
//! shipped encoding — "when everything but its `code_word` line builds,
//! tranche 0 is done by demonstration." `code_word` itself is deliberately
//! EXCLUDED (consumer-coupled; rides the first scripted-object port).
//!
//! Mechanically: copy the game tree to a temp root, drop in the patched
//! preview beside the other badniks, build via the real CLI
//! (`--root`/`--prelude`), assert exit 0 + empty stderr.

use std::path::Path;
use std::process::Command;

fn unique_temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sigil_tranche0_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        let dst = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&p, &dst);
        } else {
            std::fs::copy(&p, &dst).unwrap();
        }
    }
}

#[test]
fn preview_compiles_with_only_the_code_word_line_patched() {
    let game = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/game"));
    let preview = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/previews/pitcher_plant_script_next.emp"
    ));

    let root = unique_temp_dir();
    copy_tree(game, &root);

    // Patch EXACTLY the code_word header line — and nothing else. The count
    // assertion catches preview drift (a second code_word use, or a changed
    // spelling, must fail here, not silently pass a weaker gate).
    let src = std::fs::read_to_string(preview).expect("read the preview");
    let needle = "(encoding: code_word, base: ObjCodeBase)";
    assert_eq!(
        src.matches(needle).count(),
        1,
        "the preview must carry exactly one code_word header line"
    );
    let patched = src.replace(needle, "(encoding: word_offsets)");
    let entry = root.join("badniks/pitcher_plant_script_next.emp");
    std::fs::write(&entry, patched).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            entry.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--prelude",
            "prelude",
        ])
        .output()
        .expect("spawn sigil");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "the patched preview must build; stderr:\n{stderr}\nstdout:\n{stdout}"
    );
    assert!(
        stderr.trim().is_empty(),
        "zero diagnostics of any severity; stderr was:\n{stderr}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
