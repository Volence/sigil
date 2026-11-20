//! A build writes nothing into the aeon tree it builds (G0b Task 6, Review Focus 4).
//!
//! The sound seams emit their artifacts into the build's own generated root, a private
//! directory outside the tree, and `embed("engine/sound/generated/...")` resolves
//! through that root. Before Task 6 they wrote about 15 files into the tree's
//! `engine/sound/generated/` on every sound-on build, `--check` included. That
//! directory is gitignored in aeon, so plain `git status --porcelain` could not see the
//! writes: this gate asks git for ignored files too, and also compares every file's
//! length and mtime, which catches a rewrite of identical bytes.
//!
//! The tree is a fresh shared clone of the reference tree's HEAD, so "status empty
//! before" is a fact about a checkout this test made: it carries no build output, no
//! `__pycache__` and no generated directory. Every child build runs with `AEON_DIR`
//! removed from its environment, so the gate also proves no build needs the variable.
//!
//! Asserted, each after its own step, on the same tree:
//!   - `sigil build --check` (sonic4, sound on);
//!   - two full `sigil build` runs of the same shape;
//!   - two full `sigil build --project games/sonic4 --shape release` runs (the
//!     game-as-data driver; it has no `--check` yet).
//!
//! Reference tree: `AEON_DIR` (the house pattern; `SIGIL_STRICT_GATE` hard-fails a
//! missing tree).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon \
//!   cargo test --release -p sigil-cli --test build_hermetic
//! ```

use sigil_harness::native;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn reference() -> Option<PathBuf> {
    sigil_harness::test_support::reference_tree_for_profile(&native::sonic4_profile(false))
}

/// A directory removed when the test ends, pass or fail.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().expect("run git");
    assert!(
        out.status.success(),
        "UNMEASURABLE: `git -C {} {}` failed, so the tree's state cannot be read:\n{}",
        dir.display(),
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("git output is UTF-8")
}

/// Every path git reports, ignored files included and each untracked file by name.
fn status(tree: &Path) -> Vec<String> {
    git(tree, &["status", "--porcelain=v1", "--ignored=matching", "--untracked-files=all"])
        .lines()
        .map(str::to_string)
        .collect()
}

/// `(length, mtime)` of every file under `tree` outside `.git`.
fn stat_tree(tree: &Path) -> BTreeMap<PathBuf, (u64, std::time::SystemTime)> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<PathBuf, (u64, std::time::SystemTime)>) {
        for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).expect("metadata");
            if meta.is_dir() {
                if path.file_name().is_some_and(|n| n == ".git") {
                    continue;
                }
                walk(&path, root, out);
            } else {
                let rel = path.strip_prefix(root).expect("under root").to_path_buf();
                out.insert(rel, (meta.len(), meta.modified().expect("mtime")));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(tree, tree, &mut out);
    out
}

fn changed(
    before: &BTreeMap<PathBuf, (u64, std::time::SystemTime)>,
    after: &BTreeMap<PathBuf, (u64, std::time::SystemTime)>,
) -> Vec<String> {
    let mut v = Vec::new();
    for (p, s) in after {
        match before.get(p) {
            None => v.push(format!("created {}", p.display())),
            Some(b) if b != s => v.push(format!("rewrote {}", p.display())),
            Some(_) => {}
        }
    }
    for p in before.keys().filter(|p| !after.contains_key(*p)) {
        v.push(format!("removed {}", p.display()));
    }
    v
}

fn remove_pycache(dir: &Path) {
    for entry in std::fs::read_dir(dir).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() && !path.is_symlink() {
            if path.file_name().is_some_and(|n| n == "__pycache__") {
                std::fs::remove_dir_all(&path).expect("remove __pycache__");
            } else if path.file_name().is_none_or(|n| n != ".git") {
                remove_pycache(&path);
            }
        }
    }
}

/// A fresh checkout of the reference tree's HEAD at `tree`: no build output, no
/// generated directory, no `__pycache__` (swept anyway, the rule for every aeon copy).
fn fresh_clone(reference: &Path, tree: &Path) {
    let head = git(reference, &["rev-parse", "HEAD"]).trim().to_string();
    let clone = Command::new("git")
        .args(["clone", "-q", "--shared", "--no-checkout"])
        .arg(reference)
        .arg(tree)
        .output()
        .expect("run git clone");
    assert!(clone.status.success(), "UNMEASURABLE: git clone failed:\n{}", String::from_utf8_lossy(&clone.stderr));
    git(tree, &["-c", "advice.detachedHead=false", "checkout", "-q", "--detach", &head]);
    remove_pycache(tree);
    assert_eq!(git(tree, &["rev-parse", "HEAD"]).trim(), head, "the clone is not at the reference HEAD");
    assert!(tree.join("games/sonic4/game.toml").is_file(), "UNMEASURABLE: the clone has no games/sonic4/game.toml");
}

/// Run `sigil <args>` with `AEON_DIR` removed: its output, or the failure's text.
fn sigil(cwd: &Path, args: &[&str], what: &str) -> Result<String, String> {
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .current_dir(cwd)
        .env_remove("AEON_DIR")
        .args(args)
        .output()
        .expect("run sigil");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if out.status.success() {
        Ok(text)
    } else {
        Err(format!("{what} failed (exit {:?}):\n{text}", out.status.code()))
    }
}

#[test]
fn check_build_writes_nothing_in_tree() {
    let Some(reference) = reference() else { return };
    let scratch = Scratch(std::env::temp_dir().join(format!("sigil_build_hermetic_{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&scratch.0);
    std::fs::create_dir_all(&scratch.0).expect("scratch dir");
    let tree = scratch.0.join("tree");
    let out = scratch.0.join("out");
    std::fs::create_dir_all(&out).expect("out dir");

    fresh_clone(&reference, &tree);

    let before = status(&tree);
    assert!(before.is_empty(), "the fresh clone is not clean before any build:\n{}", before.join("\n"));
    let stat0 = stat_tree(&tree);
    assert!(stat0.len() > 1000, "UNMEASURABLE: the clone holds only {} files", stat0.len());

    let tree_s = tree.to_str().expect("UTF-8 path");
    let project = tree.join("games/sonic4");
    let project_s = project.to_str().expect("UTF-8 path");
    let rom = out.join("s4.bin");
    let rom_s = rom.to_str().expect("UTF-8 path");
    let prom = out.join("s4.project.bin");
    let prom_s = prom.to_str().expect("UTF-8 path");

    let steps: [(&str, Vec<&str>, Option<&Path>); 5] = [
        ("sigil build --check", vec!["build", "--aeon", tree_s, "--check"], None),
        ("full build 1", vec!["build", "--aeon", tree_s, "-o", rom_s], Some(&rom)),
        ("full build 2", vec!["build", "--aeon", tree_s, "-o", rom_s], Some(&rom)),
        ("--project build 1", vec!["build", "--project", project_s, "--shape", "release", "-o", prom_s], Some(&prom)),
        ("--project build 2", vec!["build", "--project", project_s, "--shape", "release", "-o", prom_s], Some(&prom)),
    ];
    let mut failures = Vec::new();
    for (what, args, artifact) in &steps {
        // A failed build ends the run, after the tree states already gathered are reported:
        // a later step failing must not hide what an earlier one wrote.
        let text = match sigil(&out, args, what) {
            Ok(text) => text,
            Err(failure) => {
                failures.push(failure);
                break;
            }
        };
        if let Some(path) = artifact {
            let len = std::fs::metadata(path).unwrap_or_else(|e| panic!("{what} wrote no ROM at {}: {e}\n{text}", path.display())).len();
            assert!(len > 0, "{what} wrote an empty ROM");
        } else {
            assert!(text.contains("checked: "), "{what} printed no census:\n{text}");
        }
        let after = status(&tree);
        let moved = changed(&stat0, &stat_tree(&tree));
        if after.is_empty() && moved.is_empty() {
            println!("WITNESS {what}: tree unchanged");
        } else {
            failures.push(format!(
                "after {what}: git status lists {} path(s), {} file(s) changed\n  status:\n    {}\n  changed:\n    {}",
                after.len(),
                moved.len(),
                after.join("\n    "),
                moved.join("\n    ")
            ));
        }
    }
    assert!(failures.is_empty(), "a build wrote into the tree it built, or failed:\n{}", failures.join("\n"));
}

/// Every file under `dir`, relative to it, with its bytes.
fn files_under(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
                out.insert(path.strip_prefix(root).expect("under root").to_path_buf(), bytes);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn write_files(dir: &Path, files: &BTreeMap<PathBuf, Vec<u8>>) {
    for (rel, bytes) in files {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    }
}

/// `native::build_emp` (the in-process entry the port tests and `native_section` use)
/// never reads the tree's `engine/sound/generated/` (G0b Task 6 fix round 1). Its
/// sound-on lowering embeds the sound artifacts, so with the tree's mount:
///   - ABSENT: the build still succeeds (an un-rooted build fails `[embed.not-found]`);
///   - PROVISIONED with the right bytes: the same sections;
///   - POISONED (every byte inverted, same names and lengths): still the same sections
///     (an un-rooted build embeds the poison).
///
/// The right bytes come from the seams themselves, emitted into a directory outside
/// the tree and copied in. Each arm is checked and every failure reported together.
#[test]
fn build_emp_never_reads_the_tree_mount() {
    let Some(reference) = reference() else { return };
    let scratch = Scratch(std::env::temp_dir().join(format!("sigil_build_emp_hermetic_{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&scratch.0);
    std::fs::create_dir_all(&scratch.0).expect("scratch dir");
    let tree = scratch.0.join("tree");
    fresh_clone(&reference, &tree);
    let mount = tree.join(sigil_harness::native::TREE_SOUND_MOUNT);
    assert!(!mount.exists(), "UNMEASURABLE: the fresh clone already has {}", mount.display());

    let profile = native::sonic4_profile(false);
    assert!(profile.sound_on, "UNMEASURABLE: the profile must embed the sound artifacts");
    let right = {
        let dir = sigil_build::build_dir::BuildDir::create().expect("build dir");
        native::emit_generated_in(&tree, dir.generated(), None).expect("emit the sound artifacts outside the tree");
        files_under(dir.generated())
    };
    assert!(right.len() >= 10, "UNMEASURABLE: the seams emitted only {} files", right.len());

    let mut failures = Vec::new();
    let absent = native::build_emp(&tree, &profile);
    if let Err(e) = &absent {
        failures.push(format!("ABSENT mount: build_emp failed, so it read the tree's mount:\n{e}"));
    }
    write_files(&mount, &right);
    let provisioned = native::build_emp(&tree, &profile).expect("build_emp with the tree's mount provisioned");
    let poison: BTreeMap<PathBuf, Vec<u8>> =
        right.iter().map(|(rel, bytes)| (rel.clone(), bytes.iter().map(|b| !b).collect())).collect();
    write_files(&mount, &poison);
    let poisoned = native::build_emp(&tree, &profile).expect("build_emp with the tree's mount poisoned");

    if let Ok(absent) = &absent {
        if absent.sections != provisioned.sections {
            failures.push("ABSENT and PROVISIONED mounts lowered different sections".to_string());
        }
    }
    if poisoned.sections != provisioned.sections {
        let differ: Vec<&str> = poisoned
            .sections
            .iter()
            .zip(&provisioned.sections)
            .filter(|(a, b)| a != b)
            .map(|(a, _)| a.name.as_str())
            .collect();
        failures.push(format!(
            "POISONED mount changed the lowered sections ({} vs {} sections; differing: {differ:?}), so build_emp embedded the tree's files",
            poisoned.sections.len(),
            provisioned.sections.len()
        ));
    }
    assert_eq!(files_under(&mount), poison, "build_emp rewrote the tree's mount");
    assert!(failures.is_empty(), "build_emp read the tree's engine/sound/generated:\n{}", failures.join("\n\n"));
}
