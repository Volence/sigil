//! `sigil --version`, observed through the `sigil` BINARY.
//!
//! The property under test is not "a version string is printed" — it is "the
//! executable that aeon actually invokes can be asked which source it came
//! from, and the answer is true". Those differ: a build script that bakes a
//! revision and is then not re-run by cargo keeps reporting the old SHA while
//! the binary relinks around it, which is the same staleness failure one level
//! down, wearing the costume of a fix. So the load-bearing test here is the
//! HEAD-equality one — it is the assertion that goes red if the rerun triggers
//! in `build.rs` ever stop firing.
//!
//! This file reads no aeon input of any kind — no environment pointer to an
//! engine tree, no reference tree, no built ROM, no listing, no golden. It
//! drives the built binary and asks git about the checkout the test itself was
//! compiled from, and that is its whole input set.
//!
//! That is stated in this negative form deliberately. `scripts/nightly_source_gates.sh`
//! classifies every file under `crates/*/tests/` by grepping for the names of
//! those aeon inputs, and refuses to run the whole lane if a match is neither in
//! its `SOURCE_GATES` list nor derivably artifact-dependent. The detector cannot
//! read English, so a file that names an aeon input only to disclaim it is
//! indistinguishable from one that uses it — and the cost is not a false
//! positive on this file, it is the nightly backstop exiting "COULD NOT RUN".
//! Prose in `crates/*/tests/` should therefore describe aeon inputs by
//! description rather than by identifier.
//!
//! ## Residual gap, named rather than papered over
//!
//! Working-tree dirtiness is *not* cross-checkable **here**, even though
//! `build.rs` now names the closure's own paths as rerun triggers so that a
//! capture follows the sources it describes. A mismatch between the reported
//! tree state and `git status` at test time stays legitimate in both directions:
//! the tree may have been edited after the capture and inside this same cargo
//! invocation, or cleaned after it. Asserting either direction would be a flake
//! in exactly the false-clean direction, so these tests assert the *shape* of
//! the tree claim, the trigger set cargo was handed, and the banner's own
//! disclosure of the limit — and the disclosure is asserted, so it cannot be
//! quietly dropped.
//!
//! The trigger set itself is not taken on trust: the gate below reads cargo's
//! own recording of the build script's stdout, so what is asserted is the
//! directive stream cargo received rather than the banner's account of it. What
//! that still cannot show is that cargo *acted* on it, which needs a tracked
//! source edited, a rebuild, and the banner read back — and that must never run
//! inside a shared checkout under a suite that may be killed.
//! `scripts/tree_state_capture_gate.sh` is that proof, standing on its own and
//! named here so the split is visible rather than assumed away.

use std::process::Command;

/// The checkout this test was compiled from. Every expectation below is
/// derived from asking git about *this* directory at test time, never from a
/// SHA pinned in a fixture.
const REPO: &str = env!("CARGO_MANIFEST_DIR");

/// `sigil --version` stdout. A non-zero exit is a failure in itself: the
/// version banner is what a build script would call to decide whether the
/// assembler is current, so it must not need a success check bolted on.
fn version_stdout(flag: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(flag)
        .output()
        .expect("spawn sigil");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "`sigil {flag}` must exit 0; got {:?}\nstdout: {stdout}\nstderr: {stderr}",
        out.status.code()
    );
    stdout
}

/// Run git in the crate's checkout. Returns `Err` with a reason rather than an
/// empty string, so a caller cannot mistake "could not ask" for "the answer is
/// nothing".
fn git(args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(REPO)
        .output()
        .map_err(|e| format!("git unavailable: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The value of a `  <label>:  <value>` line from the banner body.
fn field(stdout: &str, label: &str) -> String {
    let prefix = format!("{label}:");
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            return rest.trim().to_string();
        }
    }
    panic!("`sigil --version` printed no `{label}:` line; got:\n{stdout}");
}

/// THE INCIDENT TEST. Ties the running binary's reported revision to the tree
/// it is being tested from.
///
/// If `build.rs` fails to re-run when HEAD moves, this binary keeps reporting
/// the revision it was first built at while cargo relinks it against newer
/// code — and that is exactly the state this assertion refuses. Note it cannot
/// be satisfied by a build script that bakes a plausible-looking constant: the
/// expectation is fetched from git at test time.
#[test]
fn version_reports_the_head_of_the_tree_it_was_built_from() {
    let head = git(&["rev-parse", "HEAD"]).unwrap_or_else(|why| {
        // Loud on unmeasurable: a cross-check that cannot run is not a pass.
        panic!(
            "cannot verify the binary's revision against this tree: {why}. \
             This test asserts provenance and has no meaningful weakened form; \
             it needs a git checkout and a `git` on PATH."
        )
    });

    let stdout = version_stdout("--version");
    let reported = field(&stdout, "revision");

    assert_eq!(
        reported, head,
        "the `sigil` binary reports revision {reported} but this checkout's HEAD is {head}. \
         Either build.rs did not re-run when HEAD moved (the rerun triggers are the fix), \
         or HEAD moved while the suite was running (re-run to distinguish).\n\
         full banner:\n{stdout}"
    );
}

/// The branch is captured from the same `.git/HEAD` the revision is, so it is
/// a second, independent reading of the same trigger — a branch switch that
/// lands on the same commit still moves HEAD, and this catches a stamp that
/// missed it.
///
/// A **detached** checkout is a first-class case here, not an edge one: both
/// gate lanes run against detached worktrees, so this expectation must be
/// derived the same way in both states or it is a flake waiting for the night
/// it runs. `symbolic-ref` fails exactly when HEAD is not a symbolic ref, and
/// the capture applies that same rule to reach the same word, so the two sides
/// agree by construction rather than by coincidence.
#[test]
fn version_reports_the_branch_this_tree_is_on() {
    // `git` here returns Err on a non-zero exit, which for `symbolic-ref` is
    // the detached signal rather than a fault. A genuine git fault is caught by
    // the revision test, which has no such fallback.
    let expected = git(&["symbolic-ref", "--quiet", "--short", "HEAD"])
        .unwrap_or_else(|_| "detached".to_string());

    let stdout = version_stdout("--version");
    assert_eq!(
        field(&stdout, "branch"),
        expected,
        "reported branch disagrees with this checkout\nfull banner:\n{stdout}"
    );
}

/// `-V` is the same banner, not a truncated one. A short flag that prints less
/// invites scripts to read the cheap one and miss the caveat.
#[test]
fn short_flag_prints_the_same_banner() {
    assert_eq!(
        version_stdout("-V"),
        version_stdout("--version"),
        "`sigil -V` and `sigil --version` must print identical output"
    );
}

/// The first line is the greppable identity: `sigil <semver> (<tag>)`, with the
/// semver derived from this package's own `CARGO_PKG_VERSION` rather than a
/// copied literal.
#[test]
fn first_line_carries_the_crate_version_and_a_revision_tag() {
    let stdout = version_stdout("--version");
    let first = stdout.lines().next().expect("banner has a first line");

    let expected_head = format!("sigil {} (", env!("CARGO_PKG_VERSION"));
    assert!(
        first.starts_with(&expected_head),
        "first line must start with `{expected_head}`; got `{first}`"
    );
    assert!(
        first.ends_with(')'),
        "first line must close its revision tag; got `{first}`"
    );

    let tag = &first[expected_head.len()..first.len() - 1];
    assert!(!tag.is_empty(), "the revision tag must never be empty; got `{first}`");
}

/// No field may render an unknown as blank. An empty value reads as "fine" to
/// a human skimming and passes a grep looking for a SHA, which is precisely the
/// confident-wrong-answer failure this feature exists to prevent.
#[test]
fn no_banner_field_is_blank_or_a_bare_placeholder() {
    let stdout = version_stdout("--version");

    for line in stdout.lines().skip(1) {
        let trimmed = line.trim_start();
        // Continuation lines of the freshness paragraph carry no label.
        let Some((label, value)) = trimmed.split_once(':') else {
            continue;
        };
        if label.contains(' ') || label.is_empty() {
            continue;
        }
        let value = value.trim();
        assert!(
            !value.is_empty(),
            "field `{label}` rendered empty, an unknown must be a word, not a blank\n{stdout}"
        );
        assert!(
            !matches!(value, "-" | "n/a" | "N/A" | "0" | "null" | "none"),
            "field `{label}` rendered the placeholder `{value}` instead of stating what it is\n{stdout}"
        );
        // A dangling em-dash means a reason was promised and not supplied.
        assert!(
            !value.ends_with('—'),
            "field `{label}` promises a reason and gives none: `{value}`\n{stdout}"
        );
    }
}

/// The tag on line one and the `tree:` line are two renderings of one fact and
/// must not disagree — a dirty build tagged with the bare short SHA would read
/// as the clean commit it was built next to.
#[test]
fn the_revision_tag_agrees_with_the_reported_tree_state() {
    let stdout = version_stdout("--version");
    let first = stdout.lines().next().expect("banner has a first line");
    let tag = first
        .rsplit_once('(')
        .map(|(_, t)| t.trim_end_matches(')').to_string())
        .expect("first line carries a parenthesised tag");

    let revision = field(&stdout, "revision");
    let tree = field(&stdout, "tree");

    if revision.starts_with("unknown") || tree.starts_with("unknown") {
        assert!(
            tag == "revision-unknown" || tag.ends_with("-tree-unknown"),
            "an undetermined revision or tree must be tagged as such; tag `{tag}`\n{stdout}"
        );
        return;
    }

    let short = tag.trim_end_matches("-dirty");
    assert!(
        revision.starts_with(short),
        "the tag's short revision `{short}` is not a prefix of `{revision}`\n{stdout}"
    );

    if tree.starts_with("dirty") {
        assert!(
            tag.ends_with("-dirty"),
            "the tree was dirty at capture but the tag `{tag}` does not say so\n{stdout}"
        );
    } else {
        assert_eq!(
            tag, short,
            "a clean tree must tag the bare revision; got `{tag}`\n{stdout}"
        );
    }
}

/// A clean tree is exactly the case where `git status --porcelain` prints
/// nothing, so a capture that reads empty output as a failed probe reports the
/// healthiest possible state as `unknown`. That is the inverse of the rule this
/// feature is built on: loud-on-unmeasurable is a duty owed to states that
/// genuinely cannot be measured, and turning a measured "clean" into "unknown"
/// spends the reader's attention on a non-problem until they stop reading it.
#[test]
fn an_empty_porcelain_reads_as_clean_not_as_unknown() {
    let stdout = version_stdout("--version");
    let tree = field(&stdout, "tree");

    assert!(
        !tree.contains("produced no output"),
        "the tree probe treated empty porcelain output as a failure; empty output IS the \
         clean answer\ntree: {tree}\n{stdout}"
    );

    // A revision proves git answered at capture time, so a `status` that could
    // not answer in the same run is not a plausible environment difference.
    if field(&stdout, "revision").starts_with("unknown") {
        return;
    }
    let porcelain = git(&[
        "--no-optional-locks",
        "status",
        "--porcelain=v1",
        "--untracked-files=normal",
    ])
    .unwrap_or_else(|why| panic!("cannot read this tree's status: {why}"));
    if porcelain.is_empty() {
        assert!(
            !tree.starts_with("unknown"),
            "this checkout is clean and git answered for the revision, so the tree state had \
             no reason to be unknown\ntree: {tree}\n{stdout}"
        );
    }
}

/// The banner must disclose which of its claims cargo re-captures and which it
/// cannot. A witness that admits a limit is a witness; one that silently claims
/// freshness it cannot back is the defect. Asserting the disclosure keeps it
/// from being dropped as noise in a later tidy-up.
#[test]
fn the_banner_discloses_what_it_cannot_track() {
    let stdout = version_stdout("--version");
    let revision = field(&stdout, "revision");

    if revision.starts_with("unknown") {
        assert!(
            stdout.contains("NO revision"),
            "a binary with no revision must say so in capitals, not merely omit it\n{stdout}"
        );
        return;
    }

    let freshness = field(&stdout, "freshness");
    assert!(
        freshness.contains("re-captured"),
        "the banner must state that the revision is re-captured\n{stdout}"
    );
    assert!(
        stdout.contains("snapshot"),
        "the banner must label the tree state a snapshot, not present it as live\n{stdout}"
    );
    assert!(
        stdout.contains("under-report"),
        "the banner must name the direction in which the tree state can be wrong\n{stdout}"
    );
    assert!(
        stdout.contains("git rev-parse HEAD"),
        "the banner must tell a reader how to check this binary against a tree\n{stdout}"
    );
}

/// The rerun triggers are named in the output, so a build that could track
/// nothing cannot present itself the same way as one that tracks HEAD. This
/// asserts what cargo was *told*; the HEAD-equality test above is what proves
/// cargo acted on it.
#[test]
fn the_banner_names_the_rerun_triggers_backing_the_revision() {
    let stdout = version_stdout("--version");
    if field(&stdout, "revision").starts_with("unknown") {
        return;
    }
    let freshness = field(&stdout, "freshness");
    assert!(
        freshness.contains("cargo tracks HEAD"),
        "a revision captured from a git checkout must name `.git/HEAD` as tracked\n{stdout}"
    );
    assert!(
        !freshness.contains("cargo tracks none"),
        "a revision was reported while nothing was tracked, that stamp cannot stay true\n{stdout}"
    );
    assert!(
        freshness.contains("manifests"),
        "the closure is derived from the manifests, so a stamp that does not track them \
         describes a graph that may since have changed shape\n{stdout}"
    );
}

/// The `tree:` detail must say WHERE the dirt is, not only how much there is.
///
/// A count of modified files cannot separate an edit to a compiled source from
/// a note left in a documentation directory, and a consumer reading only that
/// count has to treat both as reasons to distrust the assembler — which is how
/// a warning ends up on permanently and stops being read. The classification
/// lives in this line, so this is the assertion that goes red if it is ever
/// dropped back to a bare count.
///
/// Non-vacuous on a clean tree too: `freshness` names the manifests as tracked
/// only when the closure was actually derived, so a build that could not
/// classify anything fails here rather than passing quietly.
#[test]
fn the_tree_detail_says_where_the_dirt_is() {
    let stdout = version_stdout("--version");
    if field(&stdout, "revision").starts_with("unknown") {
        return;
    }

    assert!(
        field(&stdout, "freshness").contains("manifests"),
        "the closure that backs the classification was not derived, so nothing in this banner \
         separates a source change from any other change\n{stdout}"
    );

    let tree = field(&stdout, "tree");
    let state = tree.split_whitespace().next().unwrap_or_default();
    if state == "clean" {
        assert!(
            tree.contains("no uncommitted changes"),
            "a clean tree must say so rather than leave the reason blank\n{stdout}"
        );
        return;
    }
    assert!(
        tree.contains("in the sources this binary is compiled from"),
        "a dirty tree must report how much of the dirt reached the sources this binary is \
         compiled from; got `{tree}`\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// The closure: which repository paths this executable is actually compiled from
// ---------------------------------------------------------------------------

/// The banner's `closure-paths`, split on whitespace.
fn closure_paths(stdout: &str) -> Vec<String> {
    let raw = field(stdout, "closure-paths");
    assert_ne!(
        raw, "unknown",
        "this binary could not determine what it is compiled from\n{stdout}"
    );
    raw.split_whitespace().map(|s| s.to_string()).collect()
}

/// Cargo's answer to the same question, asked again at test time.
///
/// This is deliberately a second reading of the same authority rather than a
/// second opinion: the risk being closed is a *stale* stamp — a build script
/// that did not re-run when the graph changed shape keeps describing an older
/// binary, which is the original incident one level down. What it cannot catch
/// is a rule that is wrong in both places; the rule's own edge cases are held
/// by the unit tests beside the classifier.
fn cargo_closure() -> (Vec<String>, usize) {
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--offline", "--format-version", "1"])
        .current_dir(REPO)
        .output()
        .unwrap_or_else(|e| panic!("cannot ask cargo for the dependency graph: {e}"));
    assert!(
        out.status.success(),
        "cargo metadata failed, so this gate could not measure anything: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let meta: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("cargo metadata is JSON");

    let root = std::path::PathBuf::from(
        meta["workspace_root"].as_str().expect("metadata names a workspace root"),
    );
    let packages = meta["packages"].as_array().expect("metadata lists packages");
    let find = |name: &str| {
        packages
            .iter()
            .find(|p| p["name"].as_str() == Some(name))
            .unwrap_or_else(|| panic!("cargo metadata does not list package {name}"))
    };

    let mut reached: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut queue = vec![env!("CARGO_PKG_NAME").to_string()];
    while let Some(name) = queue.pop() {
        if !reached.insert(name.clone()) {
            continue;
        }
        for dep in find(&name)["dependencies"].as_array().into_iter().flatten() {
            // Absent kind is a normal dependency; "build" is a build one. Both
            // are linked into this executable, "dev" is not, and a `path` is
            // what makes a dependency a source in this repository.
            let linked = matches!(dep["kind"].as_str(), None | Some("build"));
            if linked && dep["path"].is_string() {
                queue.push(dep["name"].as_str().expect("a dependency has a name").to_string());
            }
        }
    }

    let rel = |p: &std::path::Path| {
        p.strip_prefix(&root)
            .unwrap_or_else(|_| panic!("{} is outside the workspace", p.display()))
            .to_string_lossy()
            .into_owned()
    };

    let mut paths: std::collections::BTreeSet<String> =
        ["Cargo.toml", "Cargo.lock", ".cargo", "rust-toolchain", "rust-toolchain.toml"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    for name in &reached {
        let pkg = find(name);
        let manifest = std::path::PathBuf::from(
            pkg["manifest_path"].as_str().expect("a package has a manifest"),
        );
        let dir = manifest.parent().expect("a manifest has a directory");
        let targets = pkg["targets"].as_array().expect("a package has targets");
        let kinds = |t: &serde_json::Value| -> Vec<String> {
            t["kind"]
                .as_array()
                .expect("a target has kinds")
                .iter()
                .map(|k| k.as_str().expect("a kind is a string").to_string())
                .collect()
        };
        if targets.iter().any(|t| kinds(t).iter().any(|k| k == "custom-build")) {
            paths.insert(rel(dir));
            continue;
        }
        paths.insert(rel(&manifest));
        for target in targets {
            if kinds(target).iter().any(|k| matches!(k.as_str(), "test" | "bench" | "example")) {
                continue;
            }
            let src = std::path::PathBuf::from(
                target["src_path"].as_str().expect("a target has a source path"),
            );
            paths.insert(rel(src.parent().expect("a source path has a directory")));
        }
    }

    // Any path an ancestor already covers is redundant; the banner prints the
    // pruned form, so prune here too or the two lists differ over nothing.
    let all: Vec<String> = paths.into_iter().collect();
    let pruned: Vec<String> = all
        .iter()
        .filter(|p| !all.iter().any(|o| o != *p && p.starts_with(&format!("{o}/"))))
        .cloned()
        .collect();
    (pruned, reached.len())
}

/// The banner's closure must be cargo's *current* answer.
///
/// A closure baked at build time and never re-derived is the same staleness
/// failure this whole banner exists to detect: it would keep describing the
/// packages the binary used to be compiled from while cargo links it against a
/// graph that has since grown. The manifests are named as rerun triggers so
/// that cannot happen, and this is the assertion that goes red if they stop
/// firing.
#[test]
fn the_closure_is_cargos_current_answer_not_a_baked_list() {
    let stdout = version_stdout("--version");
    let reported = closure_paths(&stdout);
    let (expected, packages) = cargo_closure();

    assert_eq!(
        reported, expected,
        "the binary reports a different set of compiled-from paths than cargo does now. \
         Either build.rs did not re-run when a manifest changed (the manifest rerun triggers \
         are the fix), or the two derivations disagree.\nfull banner:\n{stdout}"
    );

    let closure = field(&stdout, "closure");
    assert!(
        closure.starts_with(&format!("{packages} package(s)")),
        "the banner reports a package count that is not cargo's ({packages})\n{stdout}"
    );
}

/// `closure-revision` must be the last commit that touched those paths.
///
/// This is the value a consumer compares instead of the repository tip, and the
/// way it fails silently is by degrading into the tip — at which point the
/// warning is back to firing on every commit and saying nothing. The
/// expectation is computed from the banner's own path list at test time, so a
/// build script that printed a plausible SHA cannot satisfy it.
#[test]
fn the_closure_revision_is_the_last_commit_touching_the_closure() {
    let stdout = version_stdout("--version");
    let reported = field(&stdout, "closure-revision");
    assert!(
        !reported.starts_with("unavailable"),
        "this binary could not name the last commit that reached its own sources: {reported}\n\
         {stdout}"
    );

    let paths = closure_paths(&stdout);
    let mut args: Vec<String> =
        ["log", "-1", "--format=%H", "HEAD", "--"].iter().map(|s| s.to_string()).collect();
    // `:(top)` because git resolves a bare pathspec against the current
    // directory and this runs from the crate directory, not the repository root.
    args.extend(paths.iter().map(|p| format!(":(top){p}")));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let expected = git(&refs).unwrap_or_else(|why| {
        panic!("cannot verify the closure revision against this tree: {why}")
    });

    assert_eq!(
        reported, expected,
        "the reported closure revision is not the last commit touching the reported closure. \
         A closure revision that has quietly become the repository tip warns on every commit \
         and therefore warns about nothing.\nfull banner:\n{stdout}"
    );

    let head = git(&["rev-parse", "HEAD"]).expect("this checkout has a HEAD");
    assert_eq!(
        expected.len(),
        head.len(),
        "the closure revision must be a full object name like HEAD is\n{stdout}"
    );
}

/// The closure narrows a package to its declared source directories, which is
/// only sound while nothing in those directories reaches a file outside them.
/// `#[path]`, `include!`, `include_str!` and `include_bytes!` are the ways that
/// happens, and each one silently widens what a compilation reads while the
/// closure keeps reporting the old, narrower set — a false "cannot affect this
/// binary", which is the one wrong answer this feature must never give.
///
/// A hit inside a target the closure already excludes is fine: that target is
/// not compiled into this executable, so what it reaches cannot be either.
#[test]
fn no_compiled_source_reaches_a_file_outside_the_closure() {
    let stdout = version_stdout("--version");
    let paths = closure_paths(&stdout);
    let root = std::path::PathBuf::from(
        git(&["rev-parse", "--show-toplevel"]).expect("this checkout has a root"),
    );

    let covers = |rel: &str| paths.iter().any(|p| rel == p || rel.starts_with(&format!("{p}/")));

    // The closure widens a package carrying a build script to its whole
    // directory, which sweeps in that package's own test and example targets.
    // Those are not compiled into this executable, so what they reach cannot be
    // either, and holding them to the closure would report a false escape.
    let (skip_dirs, skip_files) = targets_not_in_this_binary();
    let skipped = |rel: &str| {
        skip_files.iter().any(|f| f == rel)
            || skip_dirs.iter().any(|d| rel.starts_with(&format!("{d}/")))
    };

    let mut escapes: Vec<String> = Vec::new();
    for rel in &paths {
        let dir = root.join(rel);
        if !dir.is_dir() {
            continue;
        }
        for file in rust_sources(&dir) {
            let file_rel = file
                .strip_prefix(&root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            if skipped(&file_rel) {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            for referenced in referenced_paths(&text) {
                let Some(parent) = file.parent() else { continue };
                let target = normalise(&parent.join(&referenced));
                let Ok(target_rel) = target.strip_prefix(&root) else {
                    escapes.push(format!(
                        "{} reaches {referenced}, which is outside the repository",
                        file.display()
                    ));
                    continue;
                };
                let target_rel = target_rel.to_string_lossy().replace('\\', "/");
                if !covers(&target_rel) {
                    escapes.push(format!(
                        "{} reaches {target_rel}, which the closure does not cover",
                        file.strip_prefix(&root).unwrap_or(&file).display()
                    ));
                }
            }
        }
    }

    assert!(
        escapes.is_empty(),
        "a compiled source reaches a file the closure calls unreachable, so an edit to that \
         file would be reported as harmless:\n  {}\nEither add the reached region to what the \
         closure derives, or move the reaching file out of the compiled sources.",
        escapes.join("\n  ")
    );
}

/// The targets in this workspace that are NOT compiled into the `sigil`
/// executable, as (directories, files) relative to the repository root.
///
/// Derived from cargo's target declarations, not from directory names: what is
/// compiled into this binary is its own `bin` target, every library it links,
/// and the build scripts that run for them. A test, bench or example target
/// contributes its whole directory (a helper module beside it is compiled only
/// with it); a second binary contributes just its own source file, which sits
/// inside a directory that IS compiled.
fn targets_not_in_this_binary() -> (Vec<String>, Vec<String>) {
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--offline", "--format-version", "1"])
        .current_dir(REPO)
        .output()
        .unwrap_or_else(|e| panic!("cannot ask cargo for the target list: {e}"));
    assert!(out.status.success(), "cargo metadata failed, so this gate could not measure anything");
    let meta: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("cargo metadata is JSON");
    let root = std::path::PathBuf::from(
        meta["workspace_root"].as_str().expect("metadata names a workspace root"),
    );

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for pkg in meta["packages"].as_array().expect("metadata lists packages") {
        let this_package = pkg["name"].as_str() == Some(env!("CARGO_PKG_NAME"));
        for target in pkg["targets"].as_array().expect("a package has targets") {
            let kinds: Vec<&str> = target["kind"]
                .as_array()
                .expect("a target has kinds")
                .iter()
                .filter_map(|k| k.as_str())
                .collect();
            let src = std::path::PathBuf::from(
                target["src_path"].as_str().expect("a target has a source path"),
            );
            let rel = |p: &std::path::Path| {
                p.strip_prefix(&root).map(|r| r.to_string_lossy().replace('\\', "/"))
            };
            if kinds.iter().any(|k| matches!(*k, "test" | "bench" | "example")) {
                if let Ok(dir) = rel(src.parent().expect("a source path has a directory")) {
                    dirs.push(dir);
                }
            } else if kinds.contains(&"bin")
                && !(this_package && target["name"].as_str() == Some("sigil"))
            {
                if let Ok(file) = rel(&src) {
                    files.push(file);
                }
            }
        }
    }
    dirs.sort();
    dirs.dedup();
    files.sort();
    files.dedup();
    (dirs, files)
}

/// Every `.rs` file under `dir`, recursively.
fn rust_sources(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_sources(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            found.push(path);
        }
    }
    found
}

/// The file paths a Rust source pulls in by path rather than by module name.
/// Only the forms that can reach outside a target's own directory are read;
/// each yields the quoted literal that follows it.
fn referenced_paths(text: &str) -> Vec<String> {
    const FORMS: [&str; 4] = ["#[path", "include_str!", "include_bytes!", "include!"];
    let mut found = Vec::new();
    for form in FORMS {
        let mut rest = text;
        while let Some(at) = rest.find(form) {
            rest = &rest[at + form.len()..];
            let Some(open) = rest.find('"') else { break };
            let after = &rest[open + 1..];
            let Some(close) = after.find('"') else { break };
            found.push(after[..close].to_string());
            rest = &after[close..];
        }
    }
    found
}

/// Resolve `.` and `..` textually. The referenced file may not exist on disk
/// (a test may be checking a form that is never compiled), and `canonicalize`
/// would fail there rather than answer.
fn normalise(path: &std::path::Path) -> std::path::PathBuf {
    let mut out = std::path::PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// The classification is an over-approximation of a narrow claim, and both
/// halves of that have to reach the reader. A banner that reported a closure
/// without saying which way it errs invites the reading it cannot support —
/// that an unchanged closure means an unchanged ROM — and that reading is how a
/// witness becomes worse than none.
#[test]
fn the_banner_discloses_how_the_closure_can_be_wrong() {
    let stdout = version_stdout("--version");
    if field(&stdout, "revision").starts_with("unknown") {
        return;
    }

    // The banner wraps, so a phrase can straddle a line break. Assertions on
    // its prose read the unwrapped form or they measure the wrapping.
    let prose = stdout.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        prose.contains("OVER-reports"),
        "the banner must name the direction the classification errs in\n{stdout}"
    );
    assert!(
        prose.contains("build script"),
        "the banner must name the case it widens to a whole directory\n{stdout}"
    );
    assert!(
        prose.contains("cannot affect this binary") && prose.contains("did not change"),
        "the banner must separate what the closure proves from what only a rebuild and a byte \
         compare prove\n{stdout}"
    );
    assert!(
        prose.contains("drift-check"),
        "the banner must point at the command that turns its closure revision into a check\n{stdout}"
    );
}

/// THE SECOND FAULT'S GATE. The drift check must be a command that works when it
/// is run as printed, not a recipe a reader assembles.
///
/// The recipe form was `git log -1 --format=%H HEAD -- <closure-paths>` beside a
/// space-separated path list, and its obvious assembly puts the list in a shell
/// variable. Under zsh — which does not word-split an unquoted parameter — `--
/// $PATHS` is ONE pathspec: it matches nothing, prints nothing and exits 0, so a
/// tree the check never looked at reads as a tree with no drift. An empty result
/// that reads as an answer is the same failure class as the stale tree state
/// this banner exists to prevent, one layer out in the reader.
///
/// So this gate does not inspect the command's spelling; it RUNS it, in every
/// shell available, and requires the reported closure revision back. A command
/// that silently returns nothing fails here, whatever it looks like.
#[test]
fn the_printed_drift_check_returns_the_reported_revision_in_every_shell() {
    let stdout = version_stdout("--version");
    let command = field(&stdout, "drift-check");
    assert!(
        !command.starts_with("unavailable"),
        "this binary printed no runnable drift check: {command}\n{stdout}"
    );
    let reported = field(&stdout, "closure-revision");
    assert!(
        !reported.starts_with("unavailable"),
        "there is no closure revision for the drift check to be compared against: {reported}"
    );

    // A recipe over a variable is what broke; a command carrying its own paths is
    // the fix, and a reader who is handed a variable to expand can get it wrong
    // again. Assert the shape that makes the failure unreachable, not merely the
    // behaviour of today's spelling.
    assert!(
        !command.contains('$') && !command.contains('<'),
        "the drift check must be runnable as printed, with nothing left for a reader to \
         substitute; got `{command}`"
    );

    let mut ran: Vec<&str> = Vec::new();
    for shell in ["sh", "bash", "zsh"] {
        let out = match Command::new(shell).arg("-c").arg(&command).output() {
            Ok(out) => out,
            // A shell that is not installed is not a finding; a shell that ran
            // and disagreed is. The count below is what keeps "no shell was
            // available" from passing as "every shell agreed".
            Err(_) => continue,
        };
        let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert!(
            out.status.success(),
            "the printed drift check failed under {shell}: {}\ncommand: {command}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            printed, reported,
            "the drift check printed in this banner returns `{printed}` under {shell}, not the \
             `{reported}` the banner reports. An empty or wrong answer here reads to a human as \
             `no drift`.\ncommand: {command}"
        );
        ran.push(shell);
    }
    assert!(
        !ran.is_empty(),
        "no shell on this machine could run the printed drift check, so this gate measured \
         nothing, that is a failure, not a pass.\ncommand: {command}"
    );
}

/// THE FIRST FAULT'S GATE: every closure path that exists on disk must be one
/// cargo was actually handed as a rerun trigger.
///
/// Without those triggers the tree-state capture is keyed on the revision moving
/// and on the manifests — never on the content of the sources — so an
/// uncommitted edit to a closure source relinks the binary while the capture
/// keeps its previous answer, and the banner reports `clean` over a binary built
/// from uncommitted code. A consumer's gate then opens on precisely the case it
/// exists to catch.
///
/// **This reads cargo's own recording of the build script's stdout, not the
/// banner's account of it.** Nothing in a process can observe its own directive
/// stream, so a claim computed beside the emission would survive the emission
/// being deleted — a gate of exactly the shape this whole feature argues
/// against. The path comes from the build script's `OUT_DIR`, so it follows
/// cargo's layout rather than assuming one, and a stream that cannot be read is
/// a FAILURE naming the standing substitute, never a quiet skip.
///
/// The expectation is derived: the trigger set must be exactly the closure paths
/// that exist, with the ones that do not deliberately absent — cargo reads a
/// trigger on a missing path as permanently dirty, which is the unconditional
/// rerun the design refuses.
#[test]
fn every_closure_path_that_exists_is_watched_for_edits() {
    let stdout = version_stdout("--version");
    let paths = closure_paths(&stdout);
    let root = std::path::PathBuf::from(
        git(&["rev-parse", "--show-toplevel"]).expect("this checkout has a root"),
    );

    let (present, missing): (Vec<String>, Vec<String>) =
        paths.iter().cloned().partition(|p| root.join(p).exists());

    // ── what cargo actually received ────────────────────────────────────────
    let recorded = env!("SIGIL_BUILD_SCRIPT_OUTPUT");
    assert!(
        !recorded.starts_with("unavailable"),
        "the build script could not say where cargo records its output ({recorded}), so this \
         gate cannot see the directive stream. That is a failure, not a pass, run \
         scripts/tree_state_capture_gate.sh, which proves the same property end to end."
    );
    let stream = std::fs::read_to_string(recorded).unwrap_or_else(|e| {
        panic!(
            "cannot read cargo's recording of the build script's output at {recorded}: {e}\n\
             This gate measures the rerun triggers cargo was handed; unable to read them it \
             measures nothing. Run scripts/tree_state_capture_gate.sh, which proves the same \
             property end to end, and fix this path before trusting a green here."
        )
    });
    // Both spellings: cargo accepts `cargo:` and, since 1.77, `cargo::`.
    let triggers: Vec<&str> = stream
        .lines()
        .filter_map(|l| {
            l.strip_prefix("cargo:rerun-if-changed=")
                .or_else(|| l.strip_prefix("cargo::rerun-if-changed="))
        })
        .collect();
    assert!(
        !triggers.is_empty(),
        "cargo was handed no rerun triggers at all, so this build script's answer is frozen at \
         whenever it last happened to run\nrecorded at: {recorded}"
    );

    for path in &present {
        let absolute = root.join(path).display().to_string();
        assert!(
            triggers.contains(&absolute.as_str()),
            "`{path}` is one of the sources this binary is compiled from and it exists, but \
             cargo was never handed it as a rerun trigger. The tree-state capture is therefore \
             not keyed on its content: edit it without committing and the banner keeps saying \
             the tree is clean while cargo relinks the binary around the edit.\n\
             cargo received {} trigger(s), recorded at {recorded}",
            triggers.len()
        );
    }
    for path in &missing {
        let absolute = root.join(path).display().to_string();
        assert!(
            !triggers.contains(&absolute.as_str()),
            "`{path}` does not exist, yet cargo was handed it as a rerun trigger. Cargo reads a \
             trigger on a missing path as dirty on EVERY build, which recompiles this crate and \
             relinks all of its test binaries every time, the unconditional-rerun cost this \
             design refuses on measured grounds.\nrecorded at {recorded}"
        );
    }

    // ── and the banner must account for the same set ────────────────────────
    let tracked = field(&stdout, "tree-tracked");
    let expected_head =
        format!("{}/{} closure path(s) watched for edits", present.len(), paths.len());
    assert!(
        tracked.starts_with(&expected_head),
        "the banner must say how much of its closure the tree state is keyed on. Expected it to \
         begin `{expected_head}`; got `{tracked}`\n{stdout}"
    );
    for path in &missing {
        assert!(
            tracked.contains(path.as_str()),
            "`{path}` is claimed as a source of this binary but does not exist, so cargo cannot \
             be told to watch it, and the banner does not name it as a hole: `{tracked}`\n{stdout}"
        );
    }
    if missing.is_empty() {
        assert!(
            tracked.contains("every derived path exists"),
            "nothing was unwatchable, so the banner must say so rather than leave a reader to \
             infer it: `{tracked}`\n{stdout}"
        );
    }

    let freshness = field(&stdout, "freshness");
    if present.is_empty() {
        assert!(
            !freshness.contains(",sources"),
            "no closure path could be watched, yet the banner claims source tracking\n{stdout}"
        );
    } else {
        assert!(
            freshness.contains(",sources"),
            "cargo was handed {} closure path(s) as triggers, so the banner must list `sources` \
             among what it tracks, a tree state a reader believes is revision-keyed is a tree \
             state nobody re-checks\n{stdout}",
            present.len()
        );
    }
}

/// The tree state word is what a consumer keys on, and the three states it can
/// take mean different things. Only a change to a compiled source may produce a
/// word beginning `dirty`; a change elsewhere must not, or the warning fires
/// permanently again — and a `clean` word must never appear while the closure
/// that justifies it could not be derived.
#[test]
fn the_tree_state_word_distinguishes_a_source_change_from_any_other() {
    let stdout = version_stdout("--version");
    let tree = field(&stdout, "tree");
    let state = tree.split_whitespace().next().unwrap_or_default();
    assert!(
        matches!(state, "clean" | "clean-sources" | "dirty" | "unknown"),
        "unrecognised tree state `{state}`, a consumer keys on this word\n{stdout}"
    );

    let closure = field(&stdout, "closure");
    if closure.contains("NOT DERIVED") || closure.starts_with("unknown") {
        assert!(
            !state.starts_with("clean"),
            "the closure could not be derived, so nothing here can place a change outside this \
             binary's sources, yet the tree reads `{state}`\n{stdout}"
        );
    }

    if state == "clean-sources" {
        assert!(
            tree.contains("none of them in the sources"),
            "a tree with changes outside the compiled sources must say so in place, not just \
             withhold the word dirty\n{stdout}"
        );
    }
}

// ── the remote anchor ───────────────────────────────────────────────────────

/// The remote-tracking ref and tip the `published:` line names, or `None` when the line
/// reports that none could be resolved.
///
/// Parsed from the rendered line rather than recomputed, because what a consumer reads is
/// the line; a gate that re-derives the same facts by the same route and compares them to
/// each other proves the two derivations agree and nothing about the text.
fn published_anchor(stdout: &str) -> Option<(String, String)> {
    let line = field(stdout, "published");
    if line.starts_with("unknown") {
        return None;
    }
    let after = line.split(" contained in ").nth(1).unwrap_or_else(|| {
        panic!("the published line names no ref it compared against: `{line}`")
    });
    let (name, rest) = after.split_once(" (").unwrap_or_else(|| {
        panic!("the published line names no tip for its ref: `{line}`")
    });
    let tip = rest.split(')').next().unwrap_or("").to_string();
    assert!(
        tip.len() == 40 && tip.chars().all(|c| c.is_ascii_hexdigit()),
        "the published line must name the REVISION it compared against, not just a ref: `{line}`"
    );
    Some((name.to_string(), tip))
}

/// THE DEFECT, as a gate. The banner said whether this binary was behind a tree; it never
/// said whether that tree was itself anything anybody else could see.
///
/// On a machine where every sibling repo is a peer's live working tree, the local `HEAD`
/// this binary is compared against can be ahead of, behind, or divergent from what any
/// other lane holds. The aeon lane read "behind", could not tell whether it meant behind
/// something published, and had to assemble a `crates/`-scoped diff against `origin/master`
/// by hand to turn the banner into a measurement.
///
/// The expectation is DERIVED at test time — the same question asked of git directly —
/// rather than compared against a spelling. And "unknown" is not a free pass: a run that
/// cannot resolve a remote-tracking ref must be a run where git cannot either, which is
/// asserted rather than assumed.
#[test]
fn the_published_line_states_this_revision_s_position_against_a_named_remote_ref() {
    let stdout = version_stdout("--version");
    let revision = field(&stdout, "revision");
    let line = field(&stdout, "published");

    let Some((name, tip)) = published_anchor(&stdout) else {
        // Loud on unmeasurable: the only honest reason for `unknown` is that nothing on
        // this machine names a published tip.
        for candidate in ["@{upstream}", "refs/remotes/origin/HEAD"] {
            assert!(
                git(&["rev-parse", "--verify", "--quiet", candidate]).is_err(),
                "the banner reports `{line}` while `{candidate}` resolves here, so a ref \
                 that could have been named was not"
            );
        }
        return;
    };

    assert_eq!(
        git(&["rev-parse", &name]).unwrap_or_else(|e| panic!("resolve {name}: {e}")),
        tip,
        "the tip the banner names for {name} is not the one git resolves"
    );

    // `merge-base --is-ancestor` is the same question, asked directly.
    let contained = Command::new("git")
        .args(["merge-base", "--is-ancestor", &revision, &tip])
        .current_dir(REPO)
        .status()
        .expect("git merge-base must run, this gate measures what git answers, so it \
                 cannot be skipped")
        .success();
    let says_yes = line.starts_with("yes");
    assert_eq!(
        says_yes, contained,
        "the banner says `{line}` while git says contained={contained} for {revision} in {name}"
    );

    // And it must not read as an alarm when it is merely a position: unpushed work is the
    // ordinary state of a lane, and a line that scolds correct work gets deleted.
    if !contained {
        assert!(
            line.contains("not a fault"),
            "an unpublished revision is the ordinary state of work in progress and the line \
             must say so rather than read as a warning: `{line}`"
        );
    }
    // Whichever way it went, the reader must be told the ref is a cache — otherwise the
    // line invites the same wrong conclusion in the other direction.
    assert!(
        line.contains("`git fetch`"),
        "the line must name the tracking ref as a LOCAL cache and what refreshes it: `{line}`"
    );
}

/// The remote-anchored drift check is RUN, not inspected — the same bar the HEAD-anchored
/// one is held to, and for the same reason: a command that silently returns nothing reads
/// to a human as "no drift" on a tree it never looked at.
#[test]
fn the_published_drift_check_runs_and_is_anchored_at_the_named_ref() {
    let stdout = version_stdout("--version");
    let command = field(&stdout, "drift-check-published");

    let Some((name, _tip)) = published_anchor(&stdout) else {
        assert!(
            command.starts_with("unavailable"),
            "no remote ref could be named, so there is no command to print: `{command}`"
        );
        return;
    };

    assert!(
        !command.starts_with("unavailable"),
        "a ref was named in `published`, so this check must be runnable: {command}\n{stdout}"
    );
    assert!(
        command.contains(&format!(" {name} --")),
        "the command must be anchored at {name}, the ref the `published` line names, and \
         say so in the command itself: {command}"
    );
    assert!(
        !command.contains('$') && !command.contains('<'),
        "it must be runnable as printed, with nothing left for a reader to substitute: \
         `{command}`"
    );

    let mut ran: Vec<&str> = Vec::new();
    for shell in ["sh", "bash", "zsh"] {
        let out = match Command::new(shell).arg("-c").arg(&command).output() {
            Ok(out) => out,
            Err(_) => continue,
        };
        assert!(
            out.status.success(),
            "the printed check failed under {shell}: {}\ncommand: {command}",
            String::from_utf8_lossy(&out.stderr)
        );
        let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert!(
            printed.len() == 40 && printed.chars().all(|c| c.is_ascii_hexdigit()),
            "the check printed `{printed}` under {shell}, not a revision. An empty answer \
             reads to a human as `no drift`.\ncommand: {command}"
        );
        // It must be answering about the NAMED ref, not about HEAD wearing its name: the
        // revision it reports has to be reachable from that ref.
        let reachable = Command::new("git")
            .args(["merge-base", "--is-ancestor", &printed, &name])
            .current_dir(REPO)
            .status()
            .expect("git merge-base must run")
            .success();
        assert!(
            reachable,
            "the check printed {printed}, which is not reachable from {name}, so it did \
             not ask about that ref.\ncommand: {command}"
        );
        ran.push(shell);
    }
    assert!(
        !ran.is_empty(),
        "no shell on this machine could run the printed check, so this gate measured \
         nothing, that is a failure, not a pass.\ncommand: {command}"
    );
}
