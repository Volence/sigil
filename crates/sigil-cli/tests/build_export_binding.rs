//! G0a Task 3: every build with `-o` writes `<stem>.build.json`, bound to the ROM by
//! the ROM's full SHA-256, in the write order of spec 4.2.
//!
//! Real `sigil build` runs over the reference tree (`AEON_DIR`; `SIGIL_STRICT_GATE`
//! hard-fails a missing tree), COPIED once, because `sigil build` rewrites files inside
//! the tree it builds. One tree serves every test; they run one at a time (`LOCK`), and
//! the one test that edits a source restores it when it ends, pass or fail.
//!
//! The schema check shells out to python3 with the vendored schema (`contract/build`).
//! A missing python3 or jsonschema FAILS the test; it never skips.

use sigil_link::export::{sha256_hex, sha256_of};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard, OnceLock};

static LOCK: Mutex<()> = Mutex::new(());
static TREE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// The shared tree copy (serialised by the returned guard), or `None` to skip.
fn tree() -> Option<(MutexGuard<'static, ()>, PathBuf)> {
    let guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let path = TREE.get_or_init(|| {
        let profile = sigil_harness::native::demo_profile(false);
        let aeon = sigil_harness::test_support::reference_tree_for_profile(&profile)?;
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_export_binding_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tree = dir.join("tree");
        sigil_harness::test_support::copy_tree(&aeon, &tree);
        Some(tree.canonicalize().unwrap())
    });
    path.clone().map(|p| (guard, p))
}

fn scratch(tree: &Path, case: &str) -> PathBuf {
    let dir = tree.parent().unwrap().join(case);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(tree: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--aeon"]).arg(tree).args(["--game", "demo"]).args(args);
    cmd.env_remove("SIGIL_TEST_FAIL_BEFORE_SIDECAR").env_remove("SIGIL_TEST_FAIL_EXPORT");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("spawn sigil")
}

fn build_ok(tree: &Path, args: &[&str]) {
    let o = run(tree, args, &[]);
    assert!(o.status.success(), "build failed:\n{}", String::from_utf8_lossy(&o.stderr));
}

fn json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("sidecar")).expect("valid json")
}

fn schema_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/build/empyrean.build-1.schema.json")
}

/// Validate `instance` against the vendored schema with python3 + jsonschema. Panics
/// (never skips) when python3 or jsonschema is missing; returns the validator's verdict.
fn schema_verdict(instance: &Path) -> Result<(), String> {
    let script = "import json,sys\nfrom jsonschema import Draft202012Validator as V\n\
        s=json.load(open(sys.argv[1]));i=json.load(open(sys.argv[2]))\n\
        errs=sorted(V(s).iter_errors(i),key=lambda e:list(e.path))\n\
        [print('/'+'/'.join(map(str,e.path)),e.message[:200]) for e in errs[:10]]\n\
        sys.exit(1 if errs else 0)\n";
    let out = Command::new("python3")
        .args(["-I", "-c", script])
        .arg(schema_path())
        .arg(instance)
        .output()
        .expect("python3 is required for the schema gate and was not found");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("ModuleNotFoundError") && !stderr.contains("ImportError"),
        "python3 `jsonschema` is required for the schema gate and is not installed: {stderr}"
    );
    match out.status.code() {
        Some(0) => Ok(()),
        Some(1) => Err(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => panic!("schema validator could not run: {stderr}"),
    }
}

#[test]
fn sidecar_beside_rom_names_its_sha256() {
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "binding");
    let with = dir.join("with.bin");
    let without = dir.join("without.bin");
    build_ok(&tree, &["-o", with.to_str().unwrap()]);
    build_ok(&tree, &["-o", without.to_str().unwrap(), "--no-build-json"]);

    let rom = std::fs::read(&with).unwrap();
    assert!(rom == std::fs::read(&without).unwrap(), "the sidecar moved ROM bytes");
    let v = json(&dir.join("with.build.json"));
    assert_eq!(v["rom"]["sha256"].as_str().unwrap(), sha256_hex(&sha256_of(&rom)));
    assert_eq!(v["rom"]["size"].as_u64().unwrap(), rom.len() as u64);
    assert_eq!(v["rom"]["path"].as_str().unwrap(), "with.bin");
    assert_eq!(v["format"], "empyrean.build");
    assert_eq!(v["version"]["major"], 1);
    assert_eq!(v["shape"]["game"], "demo");
    assert_eq!(v["shape"]["shape"], "demo plain");

    // The ROM is the one the sidecar names, and no other byte string is.
    let mut patched = rom.clone();
    patched[0x100] ^= 1;
    assert_ne!(v["rom"]["sha256"].as_str().unwrap(), sha256_hex(&sha256_of(&patched)));
}

#[test]
fn the_real_sidecar_validates_against_the_vendored_schema() {
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "schema");
    build_ok(&tree, &["-o", dir.join("s.bin").to_str().unwrap()]);
    let sidecar = dir.join("s.build.json");
    schema_verdict(&sidecar).expect("the real demo sidecar must satisfy the schema");

    // Control: the gate is not vacuous. A one-digit-short ROM hash must be refused.
    let mut v = json(&sidecar);
    let hex = v["rom"]["sha256"].as_str().unwrap().to_string();
    v["rom"]["sha256"] = serde_json::Value::String(hex[1..].to_string());
    let bad = dir.join("bad.build.json");
    std::fs::write(&bad, serde_json::to_string(&v).unwrap()).unwrap();
    let verdict = schema_verdict(&bad);
    assert!(verdict.is_err(), "a 63-digit sha256 passed the schema: the gate is blind");
}

#[test]
fn stale_sidecar_is_removed_before_install() {
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "stale");
    let rom = dir.join("r.bin");
    let sidecar = dir.join("r.build.json");
    std::fs::write(&sidecar, "{\"stale\":true}").unwrap();
    let o = run(&tree, &["-o", rom.to_str().unwrap()], &[("SIGIL_TEST_FAIL_BEFORE_SIDECAR", "1")]);
    assert!(!o.status.success(), "the failure hook must fail the build");
    assert!(String::from_utf8_lossy(&o.stderr).contains("SIGIL_TEST_FAIL_BEFORE_SIDECAR"));
    assert!(rom.exists(), "the ROM is installed before the hook point, so the case is the hard one");
    assert!(!sidecar.exists(), "a stale sidecar survived beside a newly installed ROM");
}

#[test]
fn an_export_failure_writes_nothing() {
    // The export is rendered in full before the first write (final review A-M1): a
    // refusal leaves the previous ROM, listing and sidecar exactly as they were, so they
    // still agree. Before the fix the line walk ran after the install and left a new
    // ROM and listing with no sidecar.
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "export_fail");
    let (rom, lst, sidecar) = (dir.join("e.bin"), dir.join("e.lst"), dir.join("e.build.json"));
    for (p, body) in [(&rom, "old rom"), (&lst, "old listing"), (&sidecar, "{\"old\":true}")] {
        std::fs::write(p, body).unwrap();
    }
    let o = run(
        &tree,
        &["-o", rom.to_str().unwrap(), "--emit-lst", lst.to_str().unwrap()],
        &[("SIGIL_TEST_FAIL_EXPORT", "1")],
    );
    assert!(!o.status.success(), "the export refusal must fail the build");
    assert!(String::from_utf8_lossy(&o.stderr).contains("SIGIL_TEST_FAIL_EXPORT"));
    assert_eq!(std::fs::read_to_string(&rom).unwrap(), "old rom", "a new ROM was installed before the export was stated");
    assert_eq!(std::fs::read_to_string(&lst).unwrap(), "old listing", "a new listing was installed before the export was stated");
    assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), "{\"old\":true}", "the sidecar was touched by a refused export");
}

#[test]
fn no_build_json_writes_none() {
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "none");
    let rom = dir.join("n.bin");
    build_ok(&tree, &["-o", rom.to_str().unwrap(), "--no-build-json"]);
    assert!(rom.exists());
    assert!(!dir.join("n.build.json").exists());
    // Without -o there is no ROM to bind, so no sidecar anywhere.
    build_ok(&tree, &[]);
    assert!(!tree.join("demo.build.json").exists());
    // A stale sidecar is removed even when none is asked for: it would describe a ROM
    // that was just replaced.
    std::fs::write(dir.join("n.build.json"), "{}").unwrap();
    build_ok(&tree, &["-o", rom.to_str().unwrap(), "--no-build-json"]);
    assert!(!dir.join("n.build.json").exists());
}

#[test]
fn inputs_sha256_changes_with_one_source_byte() {
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "sensitivity");
    let before = dir.join("before.bin");
    let after = dir.join("after.bin");
    build_ok(&tree, &["-o", before.to_str().unwrap()]);

    let victim = tree.join("engine/vdp.emp");
    let original = std::fs::read(&victim).unwrap();
    struct Restore(PathBuf, Vec<u8>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, &self.1);
        }
    }
    let _restore = Restore(victim.clone(), original.clone());
    let mut edited = original.clone();
    edited.extend_from_slice(b"\n// one source byte, as a comment\n");
    std::fs::write(&victim, &edited).unwrap();
    build_ok(&tree, &["-o", after.to_str().unwrap()]);

    let (a, b) = (json(&dir.join("before.build.json")), json(&dir.join("after.build.json")));
    assert_ne!(a["inputsSha256"], b["inputsSha256"], "an edited source did not move inputsSha256");
    let row = |v: &serde_json::Value| {
        v["files"].as_array().unwrap().iter().find(|f| f["root"] == "engine" && f["path"] == "vdp.emp").cloned().unwrap()
    };
    assert_ne!(row(&a)["sha256"], row(&b)["sha256"]);
    assert_eq!(row(&b)["sha256"].as_str().unwrap(), sha256_hex(&sha256_of(&edited)));
    // Exactly one file row moved.
    let moved = a["files"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["files"].as_array().unwrap())
        .filter(|(x, y)| x != y)
        .count();
    assert_eq!(moved, 1);
}

#[test]
fn inputs_sha256_changes_with_one_blob_byte() {
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "blob");
    build_ok(&tree, &["-o", dir.join("before.bin").to_str().unwrap()]);

    // `engine/data/sine.bin` is a data blob the demo build reads: not a file row.
    let victim = tree.join("engine/data/sine.bin");
    let original = std::fs::read(&victim).unwrap();
    struct Restore(PathBuf, Vec<u8>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, &self.1);
        }
    }
    let _restore = Restore(victim.clone(), original.clone());
    let mut edited = original.clone();
    let last = edited.len() - 1;
    edited[last] ^= 1;
    std::fs::write(&victim, &edited).unwrap();
    build_ok(&tree, &["-o", dir.join("after.bin").to_str().unwrap()]);

    let (a, b) = (json(&dir.join("before.build.json")), json(&dir.join("after.build.json")));
    assert_ne!(a["inputsSha256"], b["inputsSha256"], "an edited blob did not move inputsSha256");
    assert_eq!(a["files"], b["files"], "a blob is not a file row, so the table must not move");
}

#[test]
fn inputs_sha256_ignores_output_path() {
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "outpath");
    std::fs::create_dir_all(dir.join("deeper/still")).unwrap();
    let one = dir.join("one.bin");
    let two = dir.join("deeper/still/two_renamed.bin");
    build_ok(&tree, &["-o", one.to_str().unwrap()]);
    build_ok(&tree, &["-o", two.to_str().unwrap()]);
    let (a, b) = (json(&dir.join("one.build.json")), json(&dir.join("deeper/still/two_renamed.build.json")));
    assert_eq!(a["inputsSha256"], b["inputsSha256"]);
    assert_eq!(a["rom"]["sha256"], b["rom"]["sha256"]);
    assert_ne!(a["rom"]["path"], b["rom"]["path"], "the ROM file name is the one thing that may differ");
    assert_eq!(a["files"], b["files"]);
}

#[test]
fn files_have_no_absolute_paths() {
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "paths");
    build_ok(&tree, &["-o", dir.join("p.bin").to_str().unwrap()]);
    let text = std::fs::read_to_string(dir.join("p.build.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let files = v["files"].as_array().unwrap();
    assert!(files.len() > 100, "a real demo build reads far more than {} source files", files.len());
    for f in files {
        let p = f["path"].as_str().unwrap();
        assert!(!p.starts_with('/') && !p.contains('\\') && !p.contains(".."), "{p}");
        assert!(["engine", "game", "generated", "sigil"].contains(&f["root"].as_str().unwrap()));
    }
    // Nothing in the file says where the build ran.
    for needle in [tree.to_str().unwrap(), tree.parent().unwrap().to_str().unwrap(), "/home/"] {
        assert!(!text.contains(needle), "the export names a build location: {needle}");
    }

    // Per-file hashes are of the bytes on disk (the tree is unchanged since the build).
    let aeon = &tree;
    for f in files {
        let rel = f["path"].as_str().unwrap();
        let disk = match f["root"].as_str().unwrap() {
            "engine" => aeon.join("engine").join(rel),
            "game" => aeon.join("games").join(rel),
            "generated" => {
                let (area, rest) = rel.split_once('/').unwrap();
                aeon.join("engine").join(area).join("generated").join(rest)
            }
            _ => continue,
        };
        let bytes = std::fs::read(&disk).unwrap_or_else(|e| panic!("{}: {e}", disk.display()));
        assert_eq!(f["sha256"].as_str().unwrap(), sha256_hex(&sha256_of(&bytes)), "{rel}");
        assert_eq!(f["size"].as_u64().unwrap(), bytes.len() as u64, "{rel}");
    }
}

#[test]
fn lst_unchanged_by_export() {
    let Some((_g, tree)) = tree() else { return skip() };
    let dir = scratch(&tree, "lst");
    let a = dir.join("a.bin");
    build_ok(&tree, &["-o", a.to_str().unwrap(), "--emit-lst", dir.join("a.lst").to_str().unwrap()]);
    build_ok(&tree, &["-o", a.to_str().unwrap(), "--no-build-json", "--emit-lst", dir.join("b.lst").to_str().unwrap()]);
    let (with, without) = (std::fs::read(dir.join("a.lst")).unwrap(), std::fs::read(dir.join("b.lst")).unwrap());
    assert!(with == without, "the export changed the .lst (the source digest or the listing moved)");
    assert!(with.starts_with(b"  Source Digest"), "no digest section: the comparison would be vacuous");
}

fn skip() {
    eprintln!("{}no aeon reference tree", sigil_harness::test_support::SKIP_MARKER);
}
