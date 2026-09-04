//! A DRIFT VERDICT NAMES WHAT IT COMPARED AGAINST.
//!
//! `scripts/lib/sigil_tool.sh` decides whether the assembler a provisioning run is about
//! to judge corresponds to the tree it is being provisioned from, and it compares against
//! that tree's local `HEAD`. That is the right anchor for the question — the run builds
//! from this tree, so this tree's HEAD is what the binary must correspond to — and it is
//! the wrong thing to leave unnamed.
//!
//! On this machine every sibling checkout is a peer's live working tree. A local HEAD can
//! be ahead of, behind, or divergent from anything another lane can see, so a verdict of
//! `behind` is not a measurement until something says behind WHAT. The aeon lane read
//! exactly that and had to assemble a scoped diff against `origin/master` by hand.
//!
//! THE ANCHOR IS NAMED, NOT MOVED. Anchoring the refusal at the remote would refuse every
//! lane holding unpushed commits — the ordinary state of work in progress — and an
//! always-red check is worse than an always-green one: it fires on correct work, and the
//! remedy a reasonable person reaches for is deleting the guard. So the verdict keeps its
//! anchor and states it, together with where that anchor stands against the
//! remote-tracking ref, and it says in the same breath that the ref is a LOCAL cache.
//!
//! `git ls-remote` is deliberately not used: the remote is an SSH URL, so asking the
//! server blocks, needs an agent, and fails offline, inside a script that runs before
//! every provisioning.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sigil_tool() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the harness crate sits two directories inside the sigil checkout")
        .join("scripts/lib/sigil_tool.sh")
}

fn merged(out: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

/// Run git in `dir` and insist it worked: these rows are about what git resolves, so a
/// git that could not run makes the row measure nothing.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?} in {}: {e}", dir.display()));
    assert!(
        out.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repository with one commit and NO remote-tracking refs of any kind.
fn repo(tmp: &tempfile::TempDir) -> PathBuf {
    let root = tmp.path().join("repo");
    std::fs::create_dir_all(&root).expect("mkdir");
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "gate@example.invalid"]);
    git(&root, &["config", "user.name", "gate"]);
    std::fs::write(root.join("f"), "one").expect("write");
    git(&root, &["add", "f"]);
    git(&root, &["commit", "-q", "-m", "one"]);
    root
}

/// Record `refs/remotes/origin/master` at `at`, and point `origin/HEAD` at it — the state
/// a clone leaves behind, and the one a parcel branch with no upstream still has.
fn plant_tracking_ref(root: &Path, at: &str) {
    git(root, &["update-ref", "refs/remotes/origin/master", at]);
    git(root, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/master"]);
}

fn anchor(root: &Path) -> String {
    let out = Command::new("bash")
        .arg(sigil_tool())
        .arg("--anchor")
        .arg(root)
        .output()
        .expect("run sigil_tool.sh --anchor");
    assert!(out.status.success(), "the anchor must always answer:\n{}", merged(&out));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// With nothing on this machine naming a published tip, the line says so. It does not
/// invent `origin/master`, which would be a confident statement about a ref that is not
/// there — the failure this whole file is about, one level down.
#[test]
fn with_no_tracking_ref_the_anchor_says_the_remote_position_is_unknown() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = repo(&tmp);
    let head = git(&root, &["rev-parse", "HEAD"]);

    let line = anchor(&root);
    assert!(line.contains(&head), "the anchor must name the revision compared against: {line}");
    assert!(
        line.contains(&root.display().to_string()),
        "and the tree it is a revision OF: {line}"
    );
    assert!(line.contains("UNKNOWN"), "an unresolvable remote position must say so: {line}");
    assert!(
        !line.contains("origin/master"),
        "no ref exists here, so naming one would be an invention: {line}"
    );
}

/// When the revision has reached the tracking ref, the line names the ref, its tip, and
/// the fact that the ref is a local cache — so a reader who needs the real answer knows
/// the command that gets it.
#[test]
fn a_published_head_is_reported_against_the_named_ref_and_its_tip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = repo(&tmp);
    let head = git(&root, &["rev-parse", "HEAD"]);
    plant_tracking_ref(&root, &head);

    let line = anchor(&root);
    assert!(
        line.contains(&format!("contained in origin/master ({head})")),
        "the ref AND its tip must be named: {line}"
    );
    assert!(!line.contains("NOT contained"), "this HEAD is on the ref: {line}");
    assert!(
        line.contains("LOCAL remote-tracking ref") && line.contains("`git fetch`"),
        "the reader must be told this is a cache and what refreshes it: {line}"
    );
}

/// And when it has not, the line says so WITHOUT reading as a fault: unpushed commits are
/// the ordinary state of a lane, and a line that scolds correct work gets deleted.
#[test]
fn an_unpublished_head_is_reported_as_a_position_not_a_fault() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = repo(&tmp);
    let published = git(&root, &["rev-parse", "HEAD"]);
    plant_tracking_ref(&root, &published);
    std::fs::write(root.join("f"), "two").expect("write");
    git(&root, &["add", "f"]);
    git(&root, &["commit", "-q", "-m", "two"]);
    let head = git(&root, &["rev-parse", "HEAD"]);

    let line = anchor(&root);
    assert!(line.contains(&head), "the anchor is still this tree's HEAD: {line}");
    assert!(
        line.contains(&format!("NOT contained in origin/master ({published})")),
        "the ref it is not on, and where that ref stands, must both be named: {line}"
    );
    assert!(
        line.contains("not a fault"),
        "an unpublished revision is a position, not an alarm: {line}"
    );
}

// ── the verdict itself carries it ───────────────────────────────────────────

/// A stand-in assembler: a script printing the two banner fields `sigil_tool_resolve`
/// reads, so the correspondence check can be driven to BOTH outcomes in a second.
/// Without this the only demonstration is a full provisioning run, which is the shape of
/// gate nobody re-proves.
fn plant_stub_sigil(dir: &Path, closure_revision: &str) -> PathBuf {
    let bin = dir.join("sigil-stub");
    std::fs::write(
        &bin,
        format!(
            "#!/bin/bash\ncat <<'EOF'\nsigil 0.1.0 (stub)\n  closure-revision: {closure_revision}\n  closure-paths: f\nEOF\n"
        ),
    )
    .expect("write stub");
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(&bin).expect("stat").permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(&bin, perm).expect("chmod");
    bin
}

fn resolve(root: &Path, stub: &Path, ref_target: &Path) -> Output {
    Command::new("bash")
        .arg(sigil_tool())
        .arg(root)
        .arg(ref_target)
        .env("SIGIL_BIN", stub)
        .env_remove("SIGIL_BIN_CLOSURE")
        .env_remove("REF_TARGET")
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("run sigil_tool.sh")
}

/// THE DEFECT, as a gate, on the PASSING side: the verdict a reader is most likely to
/// skim is the one that must not leave the anchor implicit.
#[test]
fn the_correspondence_verdict_names_its_anchor_when_it_passes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = repo(&tmp);
    let head = git(&root, &["rev-parse", "HEAD"]);
    plant_tracking_ref(&root, &head);
    let stub = plant_stub_sigil(tmp.path(), &head);

    let out = resolve(&root, &stub, &tmp.path().join("t"));
    let text = merged(&out);
    assert!(out.status.success(), "a corresponding binary must pass:\n{text}");
    assert!(
        text.contains(&format!("compared against HEAD {head}")),
        "the passing verdict must say which revision it compared against:\n{text}"
    );
    assert!(
        text.contains("origin/master") && text.contains("`git fetch`"),
        "and where that revision stands against what anyone else can see:\n{text}"
    );
}

/// And on the refusing side, which is the one that sends somebody looking.
#[test]
fn the_correspondence_refusal_names_its_anchor_too() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = repo(&tmp);
    let head = git(&root, &["rev-parse", "HEAD"]);
    plant_tracking_ref(&root, &head);
    // A binary reporting a closure revision this tree has never had.
    let stub = plant_stub_sigil(tmp.path(), &"0".repeat(40));

    let out = resolve(&root, &stub, &tmp.path().join("t"));
    let text = merged(&out);
    assert!(!out.status.success(), "a non-corresponding binary must refuse:\n{text}");
    assert!(
        text.contains(&format!("anchored at HEAD {head}")),
        "the refusal must say which revision the mismatch is against:\n{text}"
    );
    assert!(
        text.contains("origin/master"),
        "and where that revision stands against the remote-tracking ref:\n{text}"
    );
}
