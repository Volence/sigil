//! G0a Task 1 spike gate: `SIGIL_EXPORT_SPIKE=1` writes `<output>.spike.json` and
//! moves no ROM byte; unset, no sidecar exists. The reference tree comes from the
//! house resolver (`AEON_DIR`; `SIGIL_STRICT_GATE` hard-fails a missing tree) and is
//! COPIED first, because `sigil build` rewrites files inside the tree it builds.

use std::path::{Path, PathBuf};
use std::process::Command;

fn build(tree: &Path, out: &Path, spike: bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--aeon"]).arg(tree).arg("-o").arg(out);
    if spike {
        cmd.env("SIGIL_EXPORT_SPIKE", "1");
    } else {
        cmd.env_remove("SIGIL_EXPORT_SPIKE");
    }
    let o = cmd.output().expect("spawn sigil");
    assert!(o.status.success(), "build failed:\n{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn spike_flag_writes_a_valid_sidecar_and_moves_no_rom_byte() {
    let profile = sigil_harness::native::sonic4_profile(false);
    let Some(aeon) = sigil_harness::test_support::reference_tree_for_profile(&profile) else {
        eprintln!("skip: no aeon reference tree");
        return;
    };
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_export_spike_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let tree = dir.join("tree");
    let st = Command::new("cp").arg("-a").arg(&aeon).arg(&tree).status().expect("cp");
    assert!(st.success());

    let plain = dir.join("plain.bin");
    let spiked = dir.join("spiked.bin");
    build(&tree, &plain, false);
    build(&tree, &spiked, true);

    let a = std::fs::read(&plain).unwrap();
    let b = std::fs::read(&spiked).unwrap();
    assert!(a == b, "the spike flag moved ROM bytes");
    assert!(!dir.join("plain.bin.spike.json").exists(), "sidecar written with the flag unset");

    let text = std::fs::read_to_string(dir.join("spiked.bin.spike.json")).expect("sidecar");
    assert!(text.starts_with(r#"{"format":"empyrean.build","version":{"major":1,"minor":0},"rom":"#));
    let v: serde_json::Value = serde_json::from_str(&text).expect("valid json");
    let hex = sigil_link::export::sha256_hex(&sigil_link::export::sha256_of(&b));
    assert_eq!(v["rom"]["sha256"].as_str().unwrap(), hex);
    assert_eq!(v["rom"]["size"].as_u64().unwrap(), b.len() as u64);
    assert!(!v["symbols"].as_array().unwrap().is_empty());
    assert!(text.len() < 20 * 1024 * 1024);
    let _ = std::fs::remove_dir_all(&dir);
}
