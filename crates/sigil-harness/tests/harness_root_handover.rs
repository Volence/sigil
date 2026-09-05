//! THE SPLIT, measured against the real `repin` binary.
//!
//! `refreeze` resolves which harness tree a freeze belongs to; `repin` writes `src/pins.rs`
//! into a harness tree. If those two ever resolve to different checkouts, a freeze lands
//! its blobs in one tree and its pins in another and reports success. `cargo run` makes
//! that reachable without anyone doing anything wrong: with a shared target directory it
//! will hand back a `repin` compiled in a different worktree rather than rebuild, so a
//! child resolving its own paths from `CARGO_MANIFEST_DIR` answers with that other tree.
//! `REPIN_BIN` bypasses the rebuild outright.
//!
//! These gates spawn the actual child binary with the actual argument list the parent
//! builds, from a working directory that is a DIFFERENT, perfectly valid harness tree.
//! Three candidate answers are therefore live and distinct — the tree the parent resolved,
//! the tree the child is standing in, and the tree the binary was compiled in — so any
//! answer but the parent's is a visible failure rather than a coincidence.

use std::path::{Path, PathBuf};
use std::process::Command;

use sigil_harness::harness_root::{
    repin_invocation, resolve_harness_root, ROOT_FLAG, ROOT_MARKERS, HARNESS_SUBDIR,
};

/// The real child, as cargo built it for this test.
const REPIN: &str = env!("CARGO_BIN_EXE_repin");

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?} could not run in {}: {e}, these gates measure what git and the child binary resolve, so they cannot be skipped", dir.display()));
    assert!(
        out.status.success(),
        "git {args:?} failed in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A harness tree that verifies: a git repository whose `crates/sigil-harness` carries
/// every marker the resolver requires.
fn plant_tree(root: &Path) -> PathBuf {
    let harness = root.join(HARNESS_SUBDIR);
    for m in ROOT_MARKERS {
        let p = harness.join(m);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "# fixture\n").unwrap();
    }
    std::fs::create_dir_all(harness.join("src")).unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "gate@example.invalid"]);
    git(root, &["config", "user.name", "gate"]);
    git(root, &["add", HARNESS_SUBDIR]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    harness
}

/// Run the child and return its stderr. The child stops on a missing `SIGIL_EMIT`, long
/// before it reads or writes anything — deliberately, since these gates must never put a
/// real `pins.rs` in reach. Which tree it settled on is announced BEFORE that stop, which
/// is the whole point of announcing unasked.
fn run_child(args: &[std::ffi::OsString], cwd: &Path) -> String {
    let out = Command::new(REPIN)
        .args(args)
        .current_dir(cwd)
        .env_remove("SIGIL_EMIT")
        .env_remove("SIGIL_HARNESS_ROOT")
        .output()
        .unwrap_or_else(|e| panic!("could not run the child binary {REPIN}: {e}"));
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!stderr.is_empty(), "the child said nothing at all, so nothing was measured");
    stderr
}

/// Which tree the child said it is operating on, as the child said it. Panics rather than
/// returning a default: a gate that cannot find the verdict has not measured anything, and
/// reading that as agreement is the failure mode being prevented.
fn operating_on(stderr: &str) -> String {
    for line in stderr.lines() {
        if let Some(rest) = line.split_once("operating on: ") {
            return rest.1.trim().to_string();
        }
        if let Some(rest) = line.split_once("built from and operating on the same tree: ") {
            return rest.1.trim().to_string();
        }
    }
    panic!(
        "the child never said which tree it is operating on, so this gate cannot tell \
         agreement from a split. Its output was:\n{stderr}"
    );
}

/// THE HEADLINE. The parent's resolved root and the child's are the same tree, with the
/// child standing somewhere that would have given a different answer on its own.
#[test]
fn the_child_operates_on_the_tree_the_parent_resolved_not_the_one_it_stands_in() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let parent_root = plant_tree(&base.join("parent"));
    let elsewhere = plant_tree(&base.join("elsewhere"));

    // The parent resolves its tree exactly as `refreeze` does.
    let resolved = resolve_harness_root(&parent_root, None).expect("the parent tree resolves");
    assert_eq!(resolved, parent_root);

    // NON-VACUITY, first: standing in `elsewhere` with nothing passed, the child answers
    // `elsewhere`. So the fixture really does offer a wrong answer for the gate to catch,
    // and a passing run below is the handover working rather than the two trees agreeing.
    assert_eq!(
        operating_on(&run_child(&[], &elsewhere)),
        elsewhere.display().to_string(),
        "with no root passed the child derives its own tree, if this fails the gate below \
         proves nothing"
    );

    // Now the parent's own argument list, run from `elsewhere`.
    for repin_bin in [None, Some(std::ffi::OsString::from(REPIN))] {
        let inv = repin_invocation(&resolved, repin_bin.clone());
        // The cargo shape's arguments are for cargo; the child receives what follows `--`.
        let child_args: Vec<std::ffi::OsString> = match &repin_bin {
            Some(_) => inv.args.clone(),
            None => {
                let sep = inv.args.iter().position(|a| a == "--").expect("cargo shape needs `--`");
                inv.args[sep + 1..].to_vec()
            }
        };
        let said = operating_on(&run_child(&child_args, &elsewhere));
        assert_eq!(
            said,
            resolved.display().to_string(),
            "parent and child resolved DIFFERENT trees, a freeze would split across them"
        );
        assert_ne!(
            said,
            elsewhere.display().to_string(),
            "the child derived from its own working directory instead of honouring the parent"
        );
    }
}

/// The prebuilt-binary path is the one that skips the rebuild unconditionally, so it is
/// the one most likely to BE a binary from another tree. It carries the root too, and the
/// child announces the mismatch between where it was built and where it is working.
#[test]
fn the_prebuilt_binary_path_carries_the_root_and_the_child_says_the_trees_differ() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let parent_root = plant_tree(&base.join("parent"));

    let inv = repin_invocation(&parent_root, Some(std::ffi::OsString::from(REPIN)));
    assert_eq!(inv.program, std::ffi::OsString::from(REPIN));
    let stderr = run_child(&inv.args, &base);
    assert_eq!(operating_on(&stderr), parent_root.display().to_string());

    // The child was built in this repository and is operating on a scratch tree, so the
    // difference is real and must be stated as a verdict, not left as two paths to
    // compare.
    assert!(
        stderr.contains("BUILT FROM A DIFFERENT TREE"),
        "a built-elsewhere child must say so in words: {stderr}"
    );
    assert!(stderr.contains("built from:"), "and name the tree it was built in: {stderr}");
}

/// A root that does not verify is refused BY NAME by the child, with what it resolved,
/// where that came from, and what it looked for. No file operation happens on a tree that
/// failed this check.
#[test]
fn the_child_refuses_a_root_that_does_not_verify() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let bare = base.join("bare");
    std::fs::create_dir_all(&bare).unwrap();

    let out = Command::new(REPIN)
        .arg(ROOT_FLAG)
        .arg(&bare)
        .current_dir(&base)
        .env_remove("SIGIL_HARNESS_ROOT")
        // Set, so a refusal here cannot be the missing-emitter refusal wearing a disguise.
        .env("SIGIL_EMIT", "/nonexistent/emit_sound_blob")
        .output()
        .unwrap_or_else(|e| panic!("could not run the child binary {REPIN}: {e}"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "a non-verifying root must not be accepted: {stderr}");
    assert!(stderr.contains(ROOT_FLAG), "the refusal must name the flag: {stderr}");
    assert!(
        stderr.contains(&bare.display().to_string()),
        "must say what it resolved to: {stderr}"
    );
    for m in ROOT_MARKERS {
        assert!(stderr.contains(m), "must name the marker `{m}` it expected: {stderr}");
    }
    assert!(
        !stderr.contains("SIGIL_EMIT to"),
        "the root must be settled before anything else is even considered: {stderr}"
    );
}

/// Outside any harness tree, with nothing passed, the child refuses rather than falling
/// back to the tree it was compiled in — which, being this repository's checkout, is a
/// real tree with a real `src/pins.rs` in it.
#[test]
fn the_child_told_nothing_outside_a_harness_tree_refuses_rather_than_guessing() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();

    let out = Command::new(REPIN)
        .current_dir(&base)
        .env_remove("SIGIL_HARNESS_ROOT")
        .env("SIGIL_EMIT", "/nonexistent/emit_sound_blob")
        .output()
        .unwrap_or_else(|e| panic!("could not run the child binary {REPIN}: {e}"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "there is no tree here to operate on: {stderr}");
    assert!(stderr.contains(&base.display().to_string()), "must say where it stood: {stderr}");
    assert!(
        !stderr.contains("wrote "),
        "nothing may be written when the tree could not be settled: {stderr}"
    );
}
