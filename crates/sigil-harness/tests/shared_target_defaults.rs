//! NO TOOL REACHES INTO A SHARED BUILD DIRECTORY WHEN IT WAS NOT TOLD TO.
//!
//! Three scripts defaulted a build directory to a path every worktree of this repo
//! shares — `scripts/provision-aeon-ref.sh` (via `scripts/lib/sigil_tool.sh`) to
//! `<sigil>/../.sigil-ref-target`, and the two golden-capture scripts to the invoking
//! checkout's own `target/`. Measured on this machine, the first of those held a `sigil`
//! reporting a branch deleted hours earlier beside rlibs compiled from a different lane's
//! tree.
//!
//! THE FAULT IS SUBSTITUTION, NOT UNTIDINESS. Cargo's unit hash is checkout-independent,
//! so a second worktree writes `deps/<name>-<hash>` carrying its own absolute
//! `CARGO_MANIFEST_DIR`; the first then matches the fingerprint, does not rebuild, and
//! runs a binary compiled against another checkout's paths. `golden/provenance.toml`
//! records that incident and its conclusion: a per-worktree target directory is the only
//! fix, and `cargo clean` is not one.
//!
//! IT IS SILENT WHERE IT MATTERS MOST. The two golden scripts CAPTURE THE GOLDENS. A
//! stale or foreign assembler selected there does not error; it produces a complete set
//! of frozen expectations attributed to the wrong binary, and every later gate reads
//! those expectations rather than the tree.
//!
//! BOTH DIRECTIONS ARE ASSERTED. A refusal proven only on the bare run is
//! indistinguishable from one that refuses everything, and an always-red check is worse
//! than an always-green one: it fires on correct work and the remedy a reasonable person
//! reaches for is deleting the guard. So every refusal row here is paired with a row
//! showing the same script getting PAST it when an input is named.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sigil_root() -> PathBuf {
    // `<sigil>/crates/sigil-harness` -> `<sigil>`. Two parents, from the compile-time
    // manifest directory rather than from cwd, which a test runner does not promise.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the harness crate sits two directories inside the sigil checkout")
        .to_path_buf()
}

fn golden_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden")
}

fn merged(out: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

/// Run `scripts/lib/sigil_tool.sh --ref-target <root>` with an environment built from
/// scratch: the variables under test are REMOVED unless a row sets them, so a value
/// leaking in from the runner cannot decide a row's answer.
fn ref_target(root: &Path, env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new("bash");
    cmd.arg(sigil_root().join("scripts/lib/sigil_tool.sh"))
        .arg("--ref-target")
        .arg(root)
        .env_remove("REF_TARGET")
        .env_remove("CARGO_TARGET_DIR");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("run sigil_tool.sh --ref-target")
}

/// The directory it answered with, insisting it answered at all.
fn resolved(out: &Output) -> String {
    assert!(
        out.status.success(),
        "the resolver refused where it was expected to answer:\n{}",
        merged(out)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

// ── (1) the build directory has no shared step ──────────────────────────────

/// THE DEFECT, as a gate: a caller who has already chosen a build directory keeps it.
///
/// `provision-aeon-ref.sh` used to compute `REF_TARGET` before consulting anything, so a
/// caller who correctly exported `CARGO_TARGET_DIR` was still sent to the suite-shared
/// directory — two lanes writing one directory, each believing it owned it.
#[test]
fn an_explicit_cargo_target_dir_is_what_the_run_builds_into() {
    let root = sigil_root();
    let out = ref_target(&root, &[("CARGO_TARGET_DIR", "/tmp/a-directory-this-caller-chose")]);
    assert_eq!(
        resolved(&out),
        "/tmp/a-directory-this-caller-chose",
        "the caller's CARGO_TARGET_DIR must be the answer, not a starting point:\n{}",
        merged(&out)
    );
}

/// `REF_TARGET` outranks it — a caller naming this script's own directory is stating an
/// intent about this run specifically, which is more particular than a build directory
/// chosen for the whole shell.
#[test]
fn ref_target_outranks_the_ambient_cargo_target_dir() {
    let root = sigil_root();
    let out = ref_target(
        &root,
        &[("CARGO_TARGET_DIR", "/tmp/ambient"), ("REF_TARGET", "/tmp/named-for-this-run")],
    );
    assert_eq!(resolved(&out), "/tmp/named-for-this-run", "{}", merged(&out));
}

/// With nothing set the answer is DERIVED FROM THE INVOKING TREE, so two worktrees can
/// never receive one directory without somebody typing it.
///
/// The expectation is derived rather than spelled: whatever the path is, it must be
/// inside the tree that asked, and it must not be the retired suite-shared one.
#[test]
fn a_bare_run_gets_a_directory_of_its_own_tree_not_a_suite_shared_one() {
    let root = sigil_root();
    let out = ref_target(&root, &[]);
    let got = resolved(&out);
    assert!(
        Path::new(&got).starts_with(&root),
        "a bare run must build inside the tree that asked ({}), got `{got}`:\n{}",
        root.display(),
        merged(&out)
    );
    assert!(
        !got.contains(".sigil-ref-target"),
        "`.sigil-ref-target` is the retired suite-shared default, one path for every \
         worktree of this repo, and a derivation must not land back on it: `{got}`"
    );
    // And the same tree asked twice answers the same, while a DIFFERENT tree answers
    // differently. Uniqueness is the whole property; a constant would satisfy the two
    // assertions above.
    let other = tempfile::tempdir().expect("tempdir");
    let elsewhere = resolved(&ref_target(other.path(), &[]));
    assert_ne!(
        got, elsewhere,
        "two different trees resolved to ONE build directory, which is the fault itself"
    );
}

/// A checkout's own `target/` is refused rather than merely not defaulted to: it is the
/// same shared artifact reached by a different route, and other lanes pin its binary by
/// hash.
#[test]
fn a_checkouts_default_target_is_refused_by_name() {
    let root = sigil_root();
    let out = ref_target(&root, &[("REF_TARGET", root.join("target").to_str().unwrap())]);
    assert!(!out.status.success(), "a shared target/ must be refused:\n{}", merged(&out));
    let text = merged(&out);
    for want in ["DEFAULT target/", "REF_TARGET", ".target-ref"] {
        assert!(text.contains(want), "the refusal must say `{want}`:\n{text}");
    }
}

/// From a LINKED WORKTREE the main checkout's `target/` is the shared one, and it is not
/// under this tree at all — so the refusal cannot be a prefix test on the caller's root.
/// This is the shape every sigil agent actually runs in.
#[test]
fn from_a_linked_worktree_the_main_checkouts_target_is_refused_too() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let main = tmp.path().join("main");
    std::fs::create_dir_all(&main).expect("mkdir main");
    let git = |dir: &Path, args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("git {args:?}: {e}, this gate measures what git resolves, so it cannot be skipped"));
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    };
    git(&main, &["init", "-q"]);
    git(&main, &["config", "user.email", "gate@example.invalid"]);
    git(&main, &["config", "user.name", "gate"]);
    std::fs::write(main.join("f"), "x").expect("write");
    git(&main, &["add", "f"]);
    git(&main, &["commit", "-q", "-m", "fixture"]);
    let wt = tmp.path().join("wt");
    git(&main, &["worktree", "add", "-q", wt.to_str().unwrap()]);

    // The control first: from the worktree, a bare run answers with the WORKTREE's own
    // directory. Without this the refusal below could be firing for any reason.
    let bare = resolved(&ref_target(&wt, &[]));
    assert!(
        Path::new(&bare).starts_with(&wt),
        "a linked worktree must build into itself, got `{bare}`"
    );

    let out = ref_target(&wt, &[("REF_TARGET", main.join("target").to_str().unwrap())]);
    assert!(
        !out.status.success(),
        "the MAIN checkout's target/ is the shared one and must be refused from a linked \
         worktree, where it is not under the caller's root at all:\n{}",
        merged(&out)
    );
}

// ── (2) the golden scripts name their assembler or stop ─────────────────────

/// A stand-in bed for the two capture scripts: the SHIPPED scripts, copied so `$HERE`
/// resolves outside the repo and no committed golden is reachable, plus the stub aeon and
/// emitter each of them insists on.
struct Bed {
    tmp: tempfile::TempDir,
}

impl Bed {
    fn plant() -> Bed {
        let bed = Bed { tmp: tempfile::tempdir().expect("tempdir") };
        std::fs::create_dir_all(bed.golden()).expect("mkdir golden");
        std::fs::create_dir_all(bed.aeon()).expect("mkdir aeon");
        std::fs::create_dir_all(bed.bin()).expect("mkdir bin");
        for s in ["capture_goldens.sh", "atomic_freeze.sh", "derive_offcanonical_sizes.sh"] {
            std::fs::copy(golden_src().join(s), bed.golden().join(s))
                .unwrap_or_else(|e| panic!("copy {s}: {e}"));
        }
        // An emitter that exists and is executable, because both scripts check for one
        // before they reach anything this file is about.
        std::fs::write(bed.bin().join("emit_sound_blob"), "#!/bin/bash\nexit 0\n").expect("write");
        let mut perm = std::fs::metadata(bed.bin().join("emit_sound_blob"))
            .expect("stat")
            .permissions();
        use std::os::unix::fs::PermissionsExt;
        perm.set_mode(0o755);
        std::fs::set_permissions(bed.bin().join("emit_sound_blob"), perm).expect("chmod");
        bed
    }

    fn golden(&self) -> PathBuf {
        self.tmp.path().join("golden")
    }
    fn aeon(&self) -> PathBuf {
        self.tmp.path().join("aeon")
    }
    fn bin(&self) -> PathBuf {
        self.tmp.path().join("bin")
    }

    /// Run one of the bed's scripts with the variables under test removed unless a row
    /// sets them.
    fn run(&self, script: &str, env: &[(&str, &str)]) -> Output {
        let mut cmd = Command::new("bash");
        cmd.arg(self.golden().join(script))
            .current_dir(self.golden())
            .env("AEON_DIR", self.aeon())
            .env("SIGIL_EMIT", self.bin().join("emit_sound_blob"))
            .env_remove("SIGIL_BUILD")
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("SIGIL_GOLDEN_WRITE");
        for (k, v) in env {
            if v.is_empty() {
                cmd.env_remove(k);
            } else {
                cmd.env(k, v);
            }
        }
        cmd.output().unwrap_or_else(|e| panic!("run {script}: {e}"))
    }
}

/// THE DEFECT, as a gate. With nothing named, the capture stops — it does not reach into
/// the shared checkout's `target/` for whichever `sigil` a lane last linked there.
#[test]
fn a_capture_with_no_assembler_named_refuses_and_says_what_it_declined() {
    let bed = Bed::plant();
    let out = bed.run("capture_goldens.sh", &[]);
    assert!(
        !out.status.success(),
        "a capture that was told nothing must stop, not pick:\n{}",
        merged(&out)
    );
    let text = merged(&out);
    // The phrase this rule and nothing else prints. Without it the row would also be
    // satisfied by the DOWNSTREAM `-x` check firing on a shared path this script had
    // already reached for, which is the defect wearing the costume of the fix.
    assert!(
        text.contains("will not pick one"),
        "the run must stop for having no assembler NAMED, not for the guessed one being \
         absent:\n{text}"
    );
    for want in ["SIGIL_BUILD", "CARGO_TARGET_DIR", "target/release/sigil"] {
        assert!(
            text.contains(want),
            "the refusal must name `{want}`, the variables consulted and the path \
             declined:\n{text}"
        );
    }
}

/// `refreeze` is the caller that ran both scripts with nothing set, so the refusals above
/// would land on the freeze ritual first. It hands its children a build directory.
///
/// MEASURED FROM THE CHILD'S ENVIRONMENT, not from refreeze's source: the capture step is
/// replaced by a stub that dumps what it was handed and stops the run there. Nothing is
/// built and no golden is reachable.
fn refreeze_child_env(extra: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("harness");
    let golden = root.join("golden");
    std::fs::create_dir_all(&golden).expect("mkdir golden");
    // The two markers that identify a harness root, and nothing else.
    std::fs::write(golden.join("provenance.toml"), "").expect("write provenance marker");
    std::fs::write(root.join("repin.toml"), "").expect("write repin marker");

    let envfile = tmp.path().join("child-env");
    let stub = golden.join("capture_goldens.sh");
    std::fs::write(&stub, format!("#!/bin/bash\nexport -p > '{}'\nexit 1\n", envfile.display()))
        .expect("write stub");
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(&stub).expect("stat").permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(&stub, perm).expect("chmod");

    // `--freeze` refuses unless it can name the aeon revision honestly.
    let aeon = tmp.path().join("aeon");
    std::fs::create_dir_all(&aeon).expect("mkdir aeon");
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(&aeon)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    };
    git(&["init", "-q"]);
    std::fs::write(aeon.join("f"), "x").expect("write");
    git(&["add", "f"]);
    git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "seed"]);

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_refreeze"));
    cmd.args(["--freeze", "gate-target-dir", "--ab", "none"])
        .current_dir(tmp.path())
        .env("SIGIL_HARNESS_ROOT", &root)
        .env("AEON_DIR", &aeon)
        .env_remove("SIGIL_GOLDEN_WRITE")
        // Removed so the DERIVATION is what is measured; a row that wants it set sets it.
        .env_remove("CARGO_TARGET_DIR");
    for (k, v) in extra {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("run refreeze");
    let dump = std::fs::read_to_string(&envfile).unwrap_or_else(|e| {
        panic!("refreeze never reached the capture step ({e}); its output was:\n{}", merged(&out))
    });
    let mut env = std::collections::BTreeMap::new();
    for line in dump.lines() {
        let Some(rest) = line.strip_prefix("declare -x ") else { continue };
        let Some((k, v)) = rest.split_once('=') else { continue };
        env.insert(k.to_string(), v.trim_matches('"').to_string());
    }
    env
}

/// With nothing set, the freeze's children are told a build directory — the one this
/// `refreeze` was itself built into, which is an observation of where it lives rather
/// than a guess, and never the shared checkout's `target/`.
#[test]
fn refreeze_tells_its_children_which_build_directory_to_use() {
    let env = refreeze_child_env(&[]);
    let got = env.get("CARGO_TARGET_DIR").unwrap_or_else(|| {
        panic!(
            "refreeze handed its capture child no build directory, so the child would have \
             to guess one, which is what these scripts now refuse. It passed: {:?}",
            env.keys().collect::<Vec<_>>()
        )
    });
    let expected = Path::new(env!("CARGO_BIN_EXE_refreeze"))
        .parent()
        .and_then(Path::parent)
        .expect("the test binary's own target directory");
    assert_eq!(
        Path::new(got),
        expected,
        "the directory handed to the child must be the one this refreeze was built into"
    );
    assert_ne!(
        Path::new(got),
        sigil_root().join("target"),
        "the shared checkout's target/ is the directory this whole file is about"
    );
}

/// And a caller who has already chosen one keeps it: an explicit `CARGO_TARGET_DIR` is a
/// statement of intent, not a hint to improve on.
#[test]
fn refreeze_passes_an_explicit_build_directory_through_unchanged() {
    let env = refreeze_child_env(&[("CARGO_TARGET_DIR", "/tmp/the-operators-own-target")]);
    assert_eq!(
        env.get("CARGO_TARGET_DIR").map(String::as_str),
        Some("/tmp/the-operators-own-target"),
        "an explicit choice must reach the child unchanged: {env:?}"
    );
}

/// The other direction, and the reason it is here: a run that names a build directory
/// gets PAST that refusal. It still fails — the bed has no assembler in it — but on the
/// binary's absence, at the path the variable implies, which is the check downstream.
#[test]
fn a_capture_given_a_build_directory_is_not_refused_for_lacking_one() {
    let bed = Bed::plant();
    let chosen = bed.tmp.path().join("chosen-target");
    let out = bed.run("capture_goldens.sh", &[("CARGO_TARGET_DIR", chosen.to_str().unwrap())]);
    let text = merged(&out);
    assert!(
        !text.contains("will not pick one"),
        "naming CARGO_TARGET_DIR must satisfy the requirement, not merely change the \
         message:\n{text}"
    );
    assert!(
        text.contains(&format!("{}/release/sigil", chosen.display())),
        "the named directory must be the one it looks in:\n{text}"
    );
}

/// Naming the binary directly satisfies it too, and needs no build directory at all —
/// this is the shape `refreeze` and the write-gate bed both use.
#[test]
fn a_capture_given_the_binary_itself_needs_no_build_directory() {
    let bed = Bed::plant();
    let out = bed.run("capture_goldens.sh", &[("SIGIL_BUILD", "/nonexistent/named/sigil")]);
    let text = merged(&out);
    assert!(!text.contains("will not pick one"), "SIGIL_BUILD alone must satisfy it:\n{text}");
    assert!(
        text.contains("/nonexistent/named/sigil"),
        "the named binary must be the one it looks for:\n{text}"
    );
}

/// The size tables are golden provenance too, and their script had the same fallback.
/// Its refusal comes FIRST — before the reference tree is resolved — so it is reachable
/// with no aeon provisioned, which is the state a reader hits it in.
#[test]
fn a_size_derivation_with_no_build_directory_refuses_before_anything_else() {
    let bed = Bed::plant();
    // AEON_DIR removed as well: if the target-dir refusal were not first, this run would
    // fail on the reference tree instead and the row would be measuring nothing.
    let out = bed.run("derive_offcanonical_sizes.sh", &[("AEON_DIR", "")]);
    assert!(!out.status.success(), "it must stop:\n{}", merged(&out));
    let text = merged(&out);
    for want in ["CARGO_TARGET_DIR", "will not pick one"] {
        assert!(text.contains(want), "the refusal must say `{want}`:\n{text}");
    }
}

/// And the control: with a build directory named it gets past that refusal and on to the
/// next unmet requirement, so the check is not simply always red.
#[test]
fn a_size_derivation_given_a_build_directory_moves_on_to_the_next_requirement() {
    let bed = Bed::plant();
    let chosen = bed.tmp.path().join("chosen-target");
    let out = bed.run(
        "derive_offcanonical_sizes.sh",
        &[("CARGO_TARGET_DIR", chosen.to_str().unwrap()), ("AEON_DIR", "")],
    );
    let text = merged(&out);
    assert!(
        !text.contains("will not pick one"),
        "naming CARGO_TARGET_DIR must satisfy the build-directory requirement:\n{text}"
    );
    assert!(
        text.contains("REFUSING, cannot locate the aeon checkout"),
        "it must proceed to the reference-tree resolution:\n{text}"
    );
}

// ── (3) the standing prohibition ────────────────────────────────────────────

/// NO SHELL SCRIPT IN THIS REPOSITORY DEFAULTS `CARGO_TARGET_DIR` TO ANYTHING.
///
/// The three sites fixed here were written independently, months apart, each by someone
/// who reasonably thought a fallback was a courtesy. The rows above hold the three that
/// exist; this holds the fourth, which will be written by someone who never read them.
///
/// A pattern gate, deliberately: the prohibition is on a SPELLING that cannot be correct,
/// so it needs no run to check and cannot be satisfied by a plausible-looking constant.
/// It fails when it finds nothing to scan, because a gate that measured no files is not a
/// gate that found no defects.
#[test]
fn no_script_supplies_a_default_for_cargo_target_dir() {
    let mut scanned = 0usize;
    let mut offenders: Vec<String> = Vec::new();
    let mut dirs = vec![sigil_root().join("scripts"), golden_src()];
    while let Some(dir) = dirs.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            // A directory that is not there is not "no defects": the gate would silently
            // stop measuring the day one is renamed.
            Err(e) => panic!("cannot scan {} ({e}), this gate measures files, so an \
                              unreadable directory is a failure, not an empty result", dir.display()),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("sh") {
                continue;
            }
            scanned += 1;
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => panic!("cannot read {} ({e})", path.display()),
            };
            for (n, line) in text.lines().enumerate() {
                let code = line.trim_start();
                // A comment quoting the retired spelling is how the reason is kept beside
                // the fix; it is prose, and gating on it would forbid explaining the rule.
                if code.starts_with('#') {
                    continue;
                }
                // `${CARGO_TARGET_DIR:-<something>}` supplies a value for a variable whose
                // whole job is to be chosen by the caller. `${CARGO_TARGET_DIR:-}` supplies
                // NOTHING — it is the presence test under `set -u`, which is how a script
                // asks whether the caller chose, and is exactly what these scripts now do.
                // The two spellings differ by one character and by everything else.
                for marker in ["CARGO_TARGET_DIR:-", "CARGO_TARGET_DIR:="] {
                    let Some(at) = code.find(marker) else { continue };
                    let rest = &code[at + marker.len()..];
                    if !rest.starts_with('}') {
                        offenders.push(format!("{}:{}: {}", path.display(), n + 1, code));
                    }
                }
            }
        }
    }
    assert!(
        scanned > 0,
        "no shell script was scanned, so this gate measured nothing, that is a failure, \
         not a pass"
    );
    assert!(
        offenders.is_empty(),
        "a build directory chosen unasked is an artifact-substitution fault, not a \
         convenience (see this file's header). Resolve it explicitly or refuse. {scanned} \
         script(s) scanned; offending line(s):\n  {}",
        offenders.join("\n  ")
    );
}
