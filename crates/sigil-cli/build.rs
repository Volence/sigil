//! Bakes the source revision this binary was built from into the executable,
//! so `sigil --version` can be *asked* what it is instead of inferred.
//!
//! Byte identity is silent on provenance: a stale assembler and a current one
//! emit byte-identical ROMs whenever the source did not change, so a matching
//! CRC is exactly as consistent with a three-day-old binary as with a fresh
//! one. The revision baked here is the only thing that distinguishes them.
//!
//! # Rerun triggers are the load-bearing part
//!
//! A provenance stamp that cargo does not re-capture when the revision moves
//! reproduces the very failure it claims to detect, one level down: the binary
//! relinks with new code while still reporting the old SHA, and now the witness
//! lies confidently. So the triggers below are what this file is *for*; the
//! string formatting is incidental.
//!
//! Emitting any `cargo:rerun-if-changed` replaces cargo's default whole-package
//! tracking for the build script, so every path this stamp depends on must be
//! named explicitly:
//!
//!   * `<git-dir>/HEAD` — moves on every checkout, and *is* the revision when
//!     HEAD is detached. In a linked worktree this lives in the worktree's own
//!     git dir, not the common dir, which is why the two are resolved apart.
//!   * `<common-dir>/refs` — tracked as a directory, which cargo walks
//!     recursively; this catches a commit rewriting a loose ref and catches a
//!     loose ref being *created* where none existed (a directory's mtime moves
//!     on file creation, so a previously packed ref going loose is not a hole).
//!   * `<common-dir>/packed-refs` — the other half of ref resolution.
//!   * every manifest in the closure below, plus the workspace `Cargo.toml` and
//!     `Cargo.lock` — these define *which* sources the closure covers, so a
//!     stamp that did not follow them would keep describing an older graph.
//!
//! Each is emitted only when it exists. A `rerun-if-changed` naming a missing
//! path makes cargo treat the unit as dirty on *every* build, which would
//! recompile this crate and relink every one of its integration-test binaries
//! each time — see the tree-state note below for why that cost is refused.
//!
//! # The dependency closure, and what a classification of it is worth
//!
//! A raw revision comparison keys on the repository tip, so any commit at all —
//! a lane-log line, a ledger row — makes an assembler look stale. That warning
//! is on permanently and therefore says nothing. The classification here asks a
//! narrower question that a tip comparison cannot: *could the drift reach this
//! executable at all?*
//!
//! The answer is derived from cargo, never listed by hand: `cargo metadata
//! --no-deps` reports each package's dependencies with their kind, and the
//! closure is the transitive non-dev path-dependency walk from this package.
//! Within each closure package the material set is narrowed by cargo's own
//! target declarations — a package's compile inputs are the module trees rooted
//! at its non-dev targets' `src_path`s, so `tests/`, `benches/` and fixture
//! directories are outside it.
//!
//! Two deliberate over-approximations, both in the safe direction:
//!
//!   * a package carrying a **build script** contributes its whole directory,
//!     because a build script may read any file in its package and nothing here
//!     models what it reads;
//!   * inside a source directory, a `#[cfg(test)]` body or a second binary
//!     counts as material although neither can reach this executable.
//!
//! And the limit of the claim: this proves "cannot affect this binary", never
//! "the output did not change". Only a rebuild and a byte compare supports the
//! second, and nothing derived here may be read as having measured it.
//!
//! What the classification is allowed to change is the `tree:` DETAIL, which is
//! free text. The state word beside it is a cross-repository interface — a
//! consumer keys on whether it begins `dirty` — so it keeps its current
//! meaning: every uncommitted change still produces `dirty`, and the reader
//! learns from the detail whether any of it reached a compiled source.
//!
//! # What cannot be tracked, and is therefore labelled a snapshot
//!
//! Working-tree dirtiness has no file whose mtime follows it. Tracking it would
//! require watching the whole repository, and the repository contains `target/`,
//! which every build mutates — so a repo-root directory trigger is not merely
//! expensive, it is a build that never reaches a fixed point. The alternative,
//! forcing the script to rerun unconditionally, was measured against cargo: a
//! rerun recompiles the dependent crate even when the script's output is
//! byte-identical, so it would pay a full recompile-and-relink of `sigil` and
//! its ~130 test binaries on every `cargo build` and every `cargo test`.
//!
//! The call: capture tree state as an explicitly-labelled snapshot and let
//! `--version` say so in its own output. `SIGIL_TREE_STATE` is truthful as of
//! the last capture and may under-report dirt if the binary was relinked
//! without HEAD moving; `SIGIL_REVISION` carries no such caveat.

use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;

// The decision half of the classification lives in the crate's own source so
// it can carry unit tests that `cargo test` runs; a build script's own code is
// never compiled into a test binary. Reached by path rather than copied,
// because two copies of a rule are two rules.
#[path = "src/tree_class.rs"]
mod tree_class;
use tree_class::{classify, state_and_detail, SourcePaths};

/// Every value the version banner renders, with a reason string when a probe
/// could not answer. Absence is always carried as a word, never as an empty
/// string — an empty field reads as "clean" to a human and to a grep.
struct Provenance {
    revision: String,
    revision_short: String,
    branch: String,
    date: String,
    tree_state: String,
    tree_detail: String,
    source_dir: String,
    tracks: String,
    error: String,
}

impl Provenance {
    /// Every field loud about being unknown, carrying `why` as the reason.
    /// `tracks` is passed through because triggers may already have been
    /// emitted by the time a later probe fails, and the banner must describe
    /// what cargo was actually told rather than what the failure implies.
    fn unknown(why: String, tracks: &str) -> Self {
        // A reasonless "unknown" is the placeholder failure in miniature: it
        // tells a reader something is wrong and gives them nothing to act on,
        // which is how a witness stops being read. There is no caller that
        // should be able to produce one.
        let why = if why.trim().is_empty() {
            "the provenance probe failed without reporting a reason".to_string()
        } else {
            why
        };
        Provenance {
            revision: "unknown".into(),
            revision_short: "unknown".into(),
            branch: "unknown".into(),
            date: "unknown".into(),
            tree_state: "unknown".into(),
            tree_detail: "not determined".into(),
            source_dir: "unknown".into(),
            tracks: tracks.into(),
            error: why,
        }
    }
}

fn main() {
    // The build script itself: emitting the git triggers below replaces
    // cargo's default package tracking, so without this an edit to this file
    // would not re-capture anything.
    println!("cargo:rerun-if-changed=build.rs");
    // The classifier this script reaches by path. It is not under cargo's
    // default package tracking either, and a stamp built from a rule that has
    // since changed is exactly the staleness this file exists to prevent.
    println!("cargo:rerun-if-changed=src/tree_class.rs");

    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
    );

    let p = probe(&manifest_dir);

    emit("SIGIL_REVISION", &p.revision);
    emit("SIGIL_REVISION_SHORT", &p.revision_short);
    emit("SIGIL_REVISION_BRANCH", &p.branch);
    emit("SIGIL_REVISION_DATE", &p.date);
    emit("SIGIL_TREE_STATE", &p.tree_state);
    emit("SIGIL_TREE_DETAIL", &p.tree_detail);
    emit("SIGIL_SOURCE_DIR", &p.source_dir);
    emit("SIGIL_REVISION_TRACKS", &p.tracks);
    emit("SIGIL_PROVENANCE_ERROR", &p.error);
}

fn emit(key: &str, value: &str) {
    // A newline here would forge additional cargo directives; a value that
    // somehow carries one is truncated at the break rather than obeyed.
    let one_line = value.split(['\n', '\r']).next().unwrap_or("");
    println!("cargo:rustc-env={key}={one_line}");
}

/// Ask git, from the crate's own directory, what tree this build is coming out
/// of. Any failure yields a fully-`unknown` result carrying the reason; a build
/// outside a git checkout must still succeed, just not claim a revision.
fn probe(manifest_dir: &Path) -> Provenance {
    let (git_dir, common_dir) = match git_dirs(manifest_dir) {
        Ok(dirs) => dirs,
        Err(why) => return Provenance::unknown(why, "none"),
    };

    let mut tracks = emit_rerun_triggers(&git_dir, &common_dir);

    let revision = match git(manifest_dir, &["rev-parse", "HEAD"]) {
        Ok(s) => s,
        Err(why) => return Provenance::unknown(why, &tracks),
    };
    let revision_short = git(manifest_dir, &["rev-parse", "--short", "HEAD"])
        .unwrap_or_else(|_| revision.chars().take(8).collect());
    // `symbolic-ref`, not `rev-parse --abbrev-ref`: the latter reports a
    // detached HEAD as the literal string "HEAD", which renders in the banner
    // as though the checkout were on a branch of that name. Both gate lanes run
    // against detached checkouts, so that is the common reading, not the exotic
    // one. `symbolic-ref` signals the state by exit status instead, leaving no
    // sentinel to misread.
    //
    // Reaching here means `rev-parse HEAD` already answered, so git works and a
    // non-zero exit means what it says: HEAD is not a symbolic ref.
    let branch = git(manifest_dir, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .unwrap_or_else(|_| "detached".into());
    let date =
        git(manifest_dir, &["log", "-1", "--format=%cI", "HEAD"]).unwrap_or_else(|_| "unknown".into());
    let source_dir =
        git(manifest_dir, &["rev-parse", "--show-toplevel"]).unwrap_or_else(|_| "unknown".into());

    let closure = closure(manifest_dir, &source_dir);
    if closure.error.is_empty() {
        // The manifests decide which sources the closure covers, so a stamp
        // that did not follow them would keep describing a graph that has since
        // changed shape. They all exist by construction — cargo just read them.
        for manifest in &closure.manifests {
            println!("cargo:rerun-if-changed={}", manifest.display());
        }
        tracks = format!("{tracks},manifests");
    }

    let (tree_state, tree_detail) = tree_status(manifest_dir, &closure);

    Provenance {
        revision,
        revision_short,
        branch,
        date,
        tree_state,
        tree_detail,
        source_dir,
        tracks,
        error: String::new(),
    }
}

/// Resolve the worktree's own git dir and the shared common dir. These differ
/// in a linked worktree — HEAD is per-worktree, refs are shared — and reading
/// only one of them loses half the revision.
fn git_dirs(manifest_dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    let out = git(
        manifest_dir,
        &["rev-parse", "--path-format=absolute", "--git-dir", "--git-common-dir"],
    )?;
    let mut lines = out.lines();
    let git_dir = lines.next().ok_or_else(|| "git rev-parse gave no git-dir".to_string())?;
    let common = lines.next().unwrap_or(git_dir);
    Ok((PathBuf::from(git_dir), PathBuf::from(common)))
}

/// Tell cargo which files move when the revision moves. Returns the
/// comma-separated list actually emitted, so the banner can state what it is
/// backed by rather than assert a guarantee it may not hold.
fn emit_rerun_triggers(git_dir: &Path, common_dir: &Path) -> String {
    let candidates: [(PathBuf, &str); 3] = [
        (git_dir.join("HEAD"), "HEAD"),
        (common_dir.join("refs"), "refs"),
        (common_dir.join("packed-refs"), "packed-refs"),
    ];

    let mut tracked: Vec<&str> = Vec::new();
    for (path, label) in candidates {
        // Only existing paths: cargo reads a missing trigger as "always
        // dirty", which is the unconditional-rerun cost this design refuses.
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
            tracked.push(label);
        }
    }

    if tracked.is_empty() {
        "none".to_string()
    } else {
        tracked.join(",")
    }
}

/// The set of repository paths this executable is compiled from, plus enough
/// context to say how it was reached and what could not be.
struct Closure {
    /// The paths themselves, and the coverage decision over them.
    sources: SourcePaths,
    /// Absolute manifest paths, emitted as rerun triggers so the walk is
    /// re-run when the graph's shape changes.
    manifests: Vec<PathBuf>,
    /// Empty when the closure was derived; otherwise why it was not, in a form
    /// a reader can act on.
    error: String,
}

impl Closure {
    fn undetermined(why: String) -> Self {
        Closure {
            sources: SourcePaths::undetermined(),
            manifests: Vec::new(),
            error: why,
        }
    }
}

/// Walk cargo's own dependency graph to the set of paths this binary compiles
/// from. Nothing here is a list of crate names — the membership is cargo's
/// answer, and the only literals are the fixed workspace-level build inputs
/// cargo's file layout defines.
fn closure(manifest_dir: &Path, repo_root: &str) -> Closure {
    let metadata = match cargo_metadata(manifest_dir) {
        Ok(v) => v,
        Err(why) => return Closure::undetermined(why),
    };

    let workspace_root = match metadata["workspace_root"].as_str() {
        Some(s) => PathBuf::from(s),
        None => return Closure::undetermined("cargo metadata reported no workspace_root".into()),
    };
    // Every pathspec below is relative to the workspace root, and the consumer
    // of `closure-paths` runs git from the repository root. When those differ
    // the pathspecs would silently address the wrong files, so this is refused
    // rather than approximated.
    if Path::new(repo_root) != workspace_root {
        return Closure::undetermined(format!(
            "the cargo workspace root {} is not the repository root {repo_root}, so \
             repository-relative pathspecs cannot be formed",
            workspace_root.display()
        ));
    }

    let packages = match metadata["packages"].as_array() {
        Some(a) => a,
        None => return Closure::undetermined("cargo metadata reported no packages".into()),
    };

    let this = std::env::var("CARGO_PKG_NAME").expect("cargo sets CARGO_PKG_NAME");
    let mut queue = VecDeque::from([this.clone()]);
    let mut reached: BTreeSet<String> = BTreeSet::new();
    while let Some(name) = queue.pop_front() {
        if !reached.insert(name.clone()) {
            continue;
        }
        let Some(pkg) = packages.iter().find(|p| p["name"].as_str() == Some(&name)) else {
            continue;
        };
        for dep in pkg["dependencies"].as_array().into_iter().flatten() {
            // `kind` is absent for a normal dependency, "build" for a build
            // dependency and "dev" for a dev one. Only the first two are linked
            // into this executable; a `path` is what makes a dependency a
            // repository source rather than a registry download.
            let kind = dep["kind"].as_str();
            if matches!(kind, None | Some("build")) && dep["path"].is_string() {
                if let Some(dep_name) = dep["name"].as_str() {
                    queue.push_back(dep_name.to_string());
                }
            }
        }
    }
    if !reached.contains(&this) {
        return Closure::undetermined(format!("cargo metadata does not list package {this}"));
    }

    // Cargo reads these regardless of which packages are involved, so they are
    // material by cargo's file layout rather than by anything about this repo.
    // Listed whether or not they exist today: a file that appears later is
    // material the moment it does, and a pathspec matching nothing is harmless.
    let mut paths: BTreeSet<String> = [
        "Cargo.toml",
        "Cargo.lock",
        ".cargo",
        "rust-toolchain",
        "rust-toolchain.toml",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let mut manifests: Vec<PathBuf> = Vec::new();
    manifests.push(workspace_root.join("Cargo.toml"));
    let lock = workspace_root.join("Cargo.lock");
    if lock.exists() {
        manifests.push(lock);
    }

    for name in &reached {
        let Some(pkg) = packages.iter().find(|p| p["name"].as_str() == Some(name.as_str())) else {
            return Closure::undetermined(format!("cargo metadata does not list package {name}"));
        };
        let Some(manifest) = pkg["manifest_path"].as_str() else {
            return Closure::undetermined(format!("package {name} reports no manifest_path"));
        };
        let manifest = PathBuf::from(manifest);
        let Some(dir) = manifest.parent() else {
            return Closure::undetermined(format!("package {name} has a rootless manifest path"));
        };
        manifests.push(manifest.clone());

        let targets = pkg["targets"].as_array().map(|v| v.as_slice()).unwrap_or(&[]);
        let has_build_script = targets.iter().any(|t| {
            t["kind"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|k| k.as_str() == Some("custom-build"))
        });

        if has_build_script {
            // A build script may read any file in its package, and cargo's own
            // default tracking for such a package is the whole directory. There
            // is nothing here that models what a given script reads, so the
            // whole directory is the only sound answer.
            match relative(&workspace_root, dir) {
                Some(rel) => {
                    paths.insert(rel);
                }
                None => {
                    return Closure::undetermined(format!(
                        "package {name} lives outside the workspace root"
                    ))
                }
            }
            continue;
        }

        match relative(&workspace_root, &manifest) {
            Some(rel) => {
                paths.insert(rel);
            }
            None => {
                return Closure::undetermined(format!(
                    "package {name}'s manifest lives outside the workspace root"
                ))
            }
        }
        for target in targets {
            let kinds: Vec<&str> = target["kind"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|k| k.as_str())
                .collect();
            // A test, bench or example target is built only by `cargo test` and
            // friends; none of them is linked into this executable.
            if kinds.iter().any(|k| matches!(*k, "test" | "bench" | "example")) {
                continue;
            }
            let Some(src) = target["src_path"].as_str() else {
                return Closure::undetermined(format!("a target of {name} reports no src_path"));
            };
            let Some(dir) = Path::new(src).parent() else {
                return Closure::undetermined(format!("a target of {name} has a rootless src_path"));
            };
            match relative(&workspace_root, dir) {
                Some(rel) => {
                    paths.insert(rel);
                }
                None => {
                    return Closure::undetermined(format!(
                        "a target of {name} lives outside the workspace root"
                    ))
                }
            }
        }
    }

    manifests.sort();
    manifests.dedup();

    Closure {
        sources: SourcePaths::derived(paths),
        manifests,
        error: String::new(),
    }
}

/// A path expressed relative to `root`, or `None` when it is not under it.
fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let rel = rel.to_str()?;
    if rel.is_empty() {
        return None;
    }
    Some(rel.replace('\\', "/"))
}

/// Ask cargo for the workspace graph. `--no-deps` keeps this to the workspace's
/// own packages, which is the whole question — a registry crate cannot be
/// edited in this checkout. `--offline` first because a build script has no
/// business reaching the network; a retry without it covers the case where the
/// lockfile genuinely needs cargo's attention, and a second failure is reported
/// rather than swallowed.
fn cargo_metadata(manifest_dir: &Path) -> Result<serde_json::Value, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest = manifest_dir.join("Cargo.toml");
    let run = |offline: bool| -> Result<Vec<u8>, String> {
        let mut cmd = Command::new(&cargo);
        cmd.args(["metadata", "--no-deps", "--format-version", "1"])
            .arg("--manifest-path")
            .arg(&manifest)
            .current_dir(manifest_dir);
        if offline {
            cmd.arg("--offline");
        }
        let out = cmd.output().map_err(|e| format!("cargo unavailable: {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let first = stderr.lines().next().unwrap_or("no stderr").trim().to_string();
            return Err(format!("cargo metadata failed: {first}"));
        }
        Ok(out.stdout)
    };

    let stdout = match run(true) {
        Ok(v) => v,
        Err(_) => run(false)?,
    };
    serde_json::from_slice(&stdout).map_err(|e| format!("cargo metadata is not valid JSON: {e}"))
}

/// Read the working tree and hand it to the classifier. `--no-optional-locks`
/// keeps a build from taking the index lock out from under a concurrent git
/// command.
fn tree_status(manifest_dir: &Path, closure: &Closure) -> (String, String) {
    // `git_allowing_empty`, not `git`: a clean tree is exactly the case where
    // porcelain output is empty, and reading that as a failed probe would
    // report the healthiest possible state as unknown.
    let out = match git_allowing_empty(
        manifest_dir,
        &["--no-optional-locks", "status", "--porcelain=v1", "--untracked-files=normal"],
    ) {
        Ok(s) => s,
        Err(why) => return ("unknown".to_string(), why),
    };

    state_and_detail(&classify(&out, &closure.sources))
}

/// Run git in `dir` for a query whose answer is a value. Empty output means the
/// probe did not answer, and an empty revision baked into the banner is the
/// blank-reads-as-fine failure this whole feature exists to avoid, so it is an
/// error here rather than a value.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let text = git_allowing_empty(dir, args)?;
    if text.is_empty() {
        return Err(format!("git {} produced no output", args.join(" ")));
    }
    Ok(text)
}

/// Run git in `dir` for a query whose answer may legitimately be empty. A
/// non-zero exit, non-UTF-8 output, or a missing git binary all become a reason
/// string; emptiness does not.
///
/// Only trailing whitespace is stripped. Leading whitespace is *data* in
/// `status --porcelain` output — the two status columns render an unstaged
/// modification as a leading space, so trimming the front shifts the first
/// entry's path left by one character and every classification of it is then
/// made against a filename that does not exist.
fn git_allowing_empty(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("git unavailable: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let first = stderr.lines().next().unwrap_or("no stderr").trim().to_string();
        return Err(format!("git {} failed: {first}", args.join(" ")));
    }
    let text = String::from_utf8(out.stdout).map_err(|_| "git output is not UTF-8".to_string())?;
    Ok(text.trim_end().to_string())
}
