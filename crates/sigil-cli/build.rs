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
//! And the limit of the claim, which the banner states in place: this proves
//! "cannot affect this binary", never "the output did not change". Only a
//! rebuild and a byte compare supports the second, and nothing derived here may
//! be read as having measured it.
//!
//! `SIGIL_CLOSURE_PATHS` and `SIGIL_CLOSURE_REVISION` carry the derivation out
//! to the banner, so a consumer can compare the last commit that reached these
//! sources instead of the repository tip — which moves on every commit and
//! therefore says nothing about any binary.
//!
//! # Tracking working-tree dirtiness: the closure's own paths
//!
//! Working-tree dirtiness has no single file whose mtime follows it, and two
//! ways of reaching for one are refused on measured grounds:
//!
//!   * **watch the whole repository** — the repository contains `target/`, which
//!     every build mutates, so a repo-root directory trigger is not merely
//!     expensive, it is a build that never reaches a fixed point;
//!   * **force the script to rerun unconditionally** — cargo recompiles the
//!     dependent crate on a rerun even when the script's output is
//!     byte-identical. Re-measured 2026-09-04 and it holds: a rerun with
//!     unchanged output costs 155 `rustc` invocations, this package's 2 bins
//!     plus its 151 integration-test targets, and that would be the price of
//!     every `cargo build` and every `cargo test`.
//!
//! Both refusals stand. Neither, however, reaches the third option, which this
//! file's own code already computes: emit a trigger for **each derived closure
//! path**. That is not the whole repository — it is the per-package source
//! directories, the manifests, and the fixed workspace-level build inputs, none
//! of which contains `target/` — and it is not an unconditional rerun, because
//! it fires only when a source the binary is compiled from actually changed,
//! which is when cargo was going to recompile this crate anyway.
//!
//! # What it costs, measured rather than argued
//!
//! Counted in `rustc` invocations under `cargo … -v`, which is the load-
//! independent quantity; `Compiling <pkg>` is printed once per PACKAGE and
//! undercounts a run that relinks 150 test binaries as `1`.
//!
//! | after                                  | with triggers | without |
//! |----------------------------------------|---------------|---------|
//! | no-op `build --release -p sigil-cli`   | 0             | 0       |
//! | no-op `test --workspace --no-run`      | 0             | 0       |
//! | edit a NON-closure file (`docs/`)      | 0             | 0       |
//! | edit a dependency source               | 347           | 347     |
//! | edit one of this package's test files  | **155**       | **1**   |
//!
//! So the no-op path is untouched — the fixed point holds, two consecutive no-op
//! builds do no work — and a real source edit costs exactly what it did, because
//! the crate was recompiling anyway. The whole bill is the last row: editing one
//! of this package's own test files now reruns the script, because `tests/` sits
//! inside `crates/sigil-cli`, which is in the closure as a whole directory
//! (a build script may read any file in its package). 0.16s becomes 4.9s.
//!
//! That row is knowingly paid rather than optimised away. Narrowing the trigger
//! set below the material set would make the tree word stale for a region the
//! classification itself calls material — a false clean, in the one direction
//! that must not exist — and would put the emitted set and the reported set out
//! of step, which is the divergence that rots. The cost lands only on the
//! edit-a-CLI-test loop, never on a no-op and never on a source edit.
//!
//! Without those triggers the capture is keyed on the revision moving and on the
//! manifests, never on the CONTENT of the sources. Cargo tracks sources for
//! compilation, so an uncommitted edit to a closure source recompiles the crate
//! and relinks the binary while this script keeps its previous answer — and the
//! banner then reports `clean` about a binary built from uncommitted code, which
//! is exactly the case a consumer's gate exists to catch. Measured on this
//! workspace, reproducibly: the binary printed the uncommitted edit back while
//! `--version` said `tree: clean at capture — no uncommitted changes`.
//!
//! The rule and its one trap live in `src/tree_class.rs` beside the classifier
//! ([`tree_class::source_triggers`]): a trigger naming a path that does not
//! exist makes cargo treat the unit dirty on every build, so only existing paths
//! are emitted — even though the very same list is *also* handed out as git
//! pathspecs, where a pathspec matching nothing is harmless.
//!
//! # What is still a snapshot
//!
//! `SIGIL_TREE_STATE` remains labelled a snapshot, because the tracking is
//! path-scoped rather than total. It can only ever under-report, and only where
//! no mtime under a watched path moves:
//!
//!   * dirt **outside** the closure — so `clean` may stand where `clean-sources`
//!     is now true. Neither word is a reason to distrust the binary, so this
//!     costs a consumer nothing;
//!   * a derived closure path that **does not exist yet** and therefore cannot be
//!     watched (`SIGIL_TREE_TRACKED` names them). In practice the two instances
//!     are `.cargo` and `rust-toolchain*`, and creating either changes rustflags
//!     or the compiler itself, which invalidates cargo's own fingerprints — but
//!     that is a mitigation, not a guarantee, and it is named here rather than
//!     relied on silently;
//!   * an edit landing inside the same cargo invocation that captured this;
//!   * any change that alters content without moving an mtime, cargo's tracking
//!     being mtime-based.
//!
//! A word beginning `dirty` is therefore trustworthy when it appears.
//! `SIGIL_REVISION` carries no such caveat.

use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;

// The decision half of the classification lives in the crate's own source so
// it can carry unit tests that `cargo test` runs; a build script's own code is
// never compiled into a test binary. Reached by path rather than copied,
// because two copies of a rule are two rules.
#[path = "src/tree_class.rs"]
mod tree_class;
use tree_class::{classify, source_triggers, state_and_detail, SourcePaths, Triggers};

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
    /// Which of the closure's paths cargo was told to watch for edits, and which
    /// could not be watched. This is what makes the residual hole in the tree
    /// state a named quantity in the output rather than a paragraph of prose.
    tree_tracked: String,
    source_dir: String,
    tracks: String,
    closure_packages: String,
    closure_paths: String,
    closure_note: String,
    closure_revision: String,
    /// The drift check as a command a reader can paste and run, with the paths
    /// already in it. See [`drift_check`] for why it is not a recipe.
    drift_check: String,
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
            tree_tracked: format!("nothing, because the revision probe failed first ({why})"),
            source_dir: "unknown".into(),
            tracks: tracks.into(),
            closure_packages: "unknown".into(),
            closure_paths: "unknown".into(),
            closure_note: format!("not derived, because the revision probe failed first ({why})"),
            closure_revision: "unavailable".into(),
            drift_check: format!("unavailable, because the revision probe failed first ({why})"),
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

    // Where cargo is recording this script's stdout, so a test can read back the
    // directives cargo ACTUALLY received rather than what the banner says about
    // them. Nothing in this process can observe its own directive stream, so
    // without this the trigger set is only assertable as a claim — and a claim
    // computed beside the emission survives the emission being deleted, which is
    // the vacuous shape of gate this whole feature exists to argue against.
    // Derived from cargo's own OUT_DIR, never a hardcoded layout.
    let build_output = std::env::var("OUT_DIR")
        .ok()
        .and_then(|out| PathBuf::from(out).parent().map(|p| p.join("output")))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unavailable — cargo set no OUT_DIR".to_string());

    let p = probe(&manifest_dir);

    emit("SIGIL_REVISION", &p.revision);
    emit("SIGIL_REVISION_SHORT", &p.revision_short);
    emit("SIGIL_REVISION_BRANCH", &p.branch);
    emit("SIGIL_REVISION_DATE", &p.date);
    emit("SIGIL_TREE_STATE", &p.tree_state);
    emit("SIGIL_TREE_DETAIL", &p.tree_detail);
    emit("SIGIL_TREE_TRACKED", &p.tree_tracked);
    emit("SIGIL_SOURCE_DIR", &p.source_dir);
    emit("SIGIL_REVISION_TRACKS", &p.tracks);
    emit("SIGIL_CLOSURE_PACKAGES", &p.closure_packages);
    emit("SIGIL_CLOSURE_PATHS", &p.closure_paths);
    emit("SIGIL_CLOSURE_NOTE", &p.closure_note);
    emit("SIGIL_CLOSURE_REVISION", &p.closure_revision);
    emit("SIGIL_DRIFT_CHECK", &p.drift_check);
    emit("SIGIL_BUILD_SCRIPT_OUTPUT", &build_output);
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

    // The tree state's own triggers. Everything above is revision-shaped, and a
    // capture keyed on the revision moving says nothing about a tree that was
    // edited without one — see the note at the top of this file. `root` is
    // `Some` exactly when the closure derived, and `closure()` has already
    // refused any workspace root that is not the repository root, so joining
    // these repository-relative paths onto it addresses the files git named.
    let tree_tracked = match closure.root.as_deref() {
        Some(root) => match source_triggers(&closure.sources, |rel| root.join(rel).exists()) {
            Triggers::Derived { emitted, absent } => {
                for rel in &emitted {
                    println!("cargo:rerun-if-changed={}", root.join(rel).display());
                }
                if !emitted.is_empty() {
                    tracks = format!("{tracks},sources");
                }
                let total = emitted.len() + absent.len();
                if absent.is_empty() {
                    format!(
                        "{}/{total} closure path(s) watched for edits; every derived path exists",
                        emitted.len()
                    )
                } else {
                    // Named, not hidden. Cargo reads a trigger on a missing path
                    // as permanently dirty, so these cannot be emitted, and a
                    // hole nothing in the output confesses is the failure this
                    // banner exists to prevent.
                    format!(
                        "{}/{total} closure path(s) watched for edits; NOT PRESENT and so \
                         unwatchable (a trigger on a missing path makes every build dirty): {}",
                        emitted.len(),
                        absent.join(" ")
                    )
                }
            }
            // Unreachable while `root` is `Some`, and answered rather than
            // asserted: an undetermined closure has no finite trigger set.
            Triggers::Undetermined => {
                "nothing — the closure could not be derived, so every path is material".to_string()
            }
        },
        None => format!(
            "nothing — the closure was not derived ({})",
            if closure.error.is_empty() { "reason not reported" } else { &closure.error }
        ),
    };

    let (tree_state, tree_detail) = tree_status(manifest_dir, &closure);

    let closure_revision = closure_revision(manifest_dir, &closure);
    let closure_packages = if closure.error.is_empty() {
        closure.packages.to_string()
    } else {
        "unknown".to_string()
    };
    let closure_note = if closure.error.is_empty() {
        format!(
            "{} path(s) derived from `cargo metadata --no-deps` at build time",
            closure.sources.paths().len()
        )
    } else {
        format!(
            "NOT DERIVED ({}) — every uncommitted change is counted as material and no \
             closure revision is available",
            closure.error
        )
    };
    let closure_paths = if closure.sources.paths().is_empty() {
        "unknown".to_string()
    } else {
        closure.sources.paths().join(" ")
    };

    let drift_check = drift_check(&source_dir, &closure);

    Provenance {
        revision,
        revision_short,
        branch,
        date,
        tree_state,
        tree_detail,
        tree_tracked,
        source_dir,
        tracks,
        closure_packages,
        closure_paths,
        closure_note,
        closure_revision,
        drift_check,
        error: String::new(),
    }
}

/// The drift check as a command, with the paths already substituted in.
///
/// The banner used to print a *recipe* — `git log -1 --format=%H HEAD --
/// <closure-paths>` — leaving a reader to assemble it from the `closure-paths`
/// line. Doing that in the obvious way puts the list in a shell variable, and
/// under zsh, which does not word-split an unquoted parameter, `-- $PATHS`
/// becomes ONE pathspec: it matches nothing, prints nothing, and exits 0. An
/// empty result reads as an answer, so the check reports "no drift" on a tree it
/// never looked at. Measured here, not theorised: with the paths correctly split
/// the same command returns exactly the revision the banner reports.
///
/// A reader who cannot assemble it wrongly is worth more than one who is told
/// how to assemble it rightly, so the command is emitted whole and needs no
/// variable, no splitting rule and no shell in particular. `-C` carries the
/// "run it at the tree root" instruction, which the pathspecs depend on, in the
/// command itself rather than in prose beside it.
fn drift_check(source_dir: &str, closure: &Closure) -> String {
    if !closure.error.is_empty() {
        return format!("unavailable — {}", closure.error);
    }
    let paths = closure.sources.paths();
    if paths.is_empty() {
        return "unavailable — no closure path was derived, so there is nothing to compare"
            .to_string();
    }
    let mut cmd = format!("git -C {} log -1 --format=%H HEAD --", shell_word(source_dir));
    for path in paths {
        cmd.push(' ');
        cmd.push_str(&shell_word(path));
    }
    cmd
}

/// One word of a shell command line. Quoted only when it needs to be, so the
/// ordinary case stays readable — but quoted whenever it does, because a path
/// carrying a space would otherwise split into two pathspecs and reintroduce the
/// silent-empty-answer failure from the other direction.
fn shell_word(word: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "._/@%+=:,-".contains(c);
    if !word.is_empty() && word.chars().all(safe) {
        return word.to_string();
    }
    format!("'{}'", word.replace('\'', r"'\''"))
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
    /// The root `sources` is relative to — `Some` exactly when the closure
    /// derived, which is also when it has been proved equal to the repository
    /// root. Carried rather than re-derived by the caller so that "these paths
    /// hang off that root" is a fact the type states instead of an assumption
    /// two functions happen to share.
    root: Option<PathBuf>,
    /// How many cargo packages the walk reached.
    packages: usize,
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
            root: None,
            packages: 0,
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
        root: Some(workspace_root),
        packages: reached.len(),
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

/// The last commit that touched anything this executable is compiled from. This
/// is the revision a consumer should compare against, because `revision` moves
/// on every commit in the repository including ones no compilation can see.
fn closure_revision(manifest_dir: &Path, closure: &Closure) -> String {
    if !closure.error.is_empty() {
        return format!("unavailable — {}", closure.error);
    }
    // `:(top)` on every pathspec: git resolves a bare pathspec against the
    // current directory, and this runs from the crate directory rather than the
    // repository root, so unprefixed paths would address files that do not
    // exist and the answer would be a confident wrong revision.
    let rooted: Vec<String> =
        closure.sources.paths().iter().map(|p| format!(":(top){p}")).collect();
    let mut args: Vec<&str> = vec!["log", "-1", "--format=%H", "HEAD", "--"];
    for path in &rooted {
        args.push(path);
    }
    match git(manifest_dir, &args) {
        Ok(sha) => sha,
        Err(why) => format!("unavailable — {why}"),
    }
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
