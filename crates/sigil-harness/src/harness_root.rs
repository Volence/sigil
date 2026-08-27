//! Which sigil harness tree a tool operates on — derived from the invoking tree, never
//! from link time, and never silently inherited from a compile-time constant.
//!
//! Both `refreeze` and its `repin` child resolve through THIS module, and `refreeze`
//! passes the root it resolved to the child on the command line ([`ROOT_FLAG`]). That
//! pairing is the point: `cargo run` is free to hand back a cached artifact built in a
//! different checkout, so a child that resolved its own paths from `CARGO_MANIFEST_DIR`
//! could write `src/pins.rs` into one tree while its parent wrote goldens into another —
//! a freeze split across two checkouts, reported as a success. A child that is TOLD the
//! root cannot disagree with the parent about it, whatever cargo decides to reuse; and a
//! child too old to understand [`ROOT_FLAG`] refuses the argument instead of running.
//!
//! A resolved root must carry every marker in [`ROOT_MARKERS`], or it is refused by name
//! with all three facts: what was resolved, where the resolution came from, and what
//! verification looked for and did not find.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The manifest directory these tools were COMPILED in. DIAGNOSTIC ONLY — it is never a
/// path they operate on, and never a fallback.
///
/// A prebuilt binary is a silent snapshot of its source at link time, and it carries that
/// snapshot in every dimension at once: its paths, its features, its flags, its defaults.
/// A stale one answers confidently in all of them. Kept here so a tool can say which tree
/// it was BUILT from beside the tree it is OPERATING on — see [`announce_root`], which
/// prints both unasked, because the people this protects have no reason to suspect the
/// binary.
pub const BUILT_FROM: &str = env!("CARGO_MANIFEST_DIR");

/// Names an explicit harness root, for the legitimate cross-tree invocation. Verified
/// exactly like a derived one: set-but-wrong is a refusal, not a licence.
pub const ROOT_OVERRIDE: &str = "SIGIL_HARNESS_ROOT";

/// The command-line spelling by which a parent tells a child tool which root to operate
/// on. An argument rather than an inherited variable, so that a child too old to know it
/// fails loudly on an unknown argument instead of quietly resolving its own root.
pub const ROOT_FLAG: &str = "--harness-root";

/// The pair that identifies a sigil harness root. BOTH are required, because either alone
/// is a weaker claim than "this is the tree whose goldens I may write".
pub const ROOT_MARKERS: [&str; 2] = ["golden/provenance.toml", "repin.toml"];

/// Where this crate sits below the repository toplevel.
pub const HARNESS_SUBDIR: &str = "crates/sigil-harness";

/// Which markers a candidate directory does NOT carry. Empty means it verifies.
pub fn missing_markers(candidate: &Path) -> Vec<&'static str> {
    ROOT_MARKERS.iter().copied().filter(|m| !candidate.join(m).is_file()).collect()
}

/// A refusal that names all three facts: what was resolved, where the resolution came
/// from, and what verification looked for and did not find. A refusal missing any of the
/// three leaves the operator to guess which tree the tool had in mind.
pub fn root_refusal(resolved: &Path, from: &str, missing: &[&str]) -> String {
    format!(
        "this is not a sigil harness root, so there is nothing here to operate on.\n  \
         resolved to:   {}\n  \
         resolved from: {from}\n  \
         expected to find, and did not: {}\n\
         Stand in the tree you mean to operate on, or set {ROOT_OVERRIDE} to its \
         {HARNESS_SUBDIR} directory. There is deliberately NO fallback to the tree this \
         binary was built from ({BUILT_FROM}): falling back there is the defect this \
         refusal exists to prevent, and it would write into that tree while you believed \
         you were working in this one.",
        resolved.display(),
        missing.join(", "),
    )
}

/// The harness root to OPERATE on, derived from the invoking tree.
///
/// `git rev-parse --show-toplevel` resolves a linked worktree to that worktree, which is
/// the property the whole derivation rests on: a freeze run from a dedicated worktree
/// must land in that worktree. Worktree isolation protects the branch pointer and gives
/// no protection at all to the working tree, so the tool has to derive the tree rather
/// than carry one from build time.
///
/// Verified before it is returned, and refused by name when it does not verify.
pub fn resolve_harness_root(cwd: &Path, override_dir: Option<&OsStr>) -> Result<PathBuf, String> {
    if let Some(dir) = override_dir {
        // Empty is a mistake, not an unset: an empty path joins the markers against the
        // working directory, so it could verify by accident from inside a harness tree
        // and then operate on a relative path.
        if dir.is_empty() {
            return Err(format!(
                "{ROOT_OVERRIDE} is set but empty. Give it the {HARNESS_SUBDIR} directory \
                 of the tree to operate on, or unset it to derive the tree you are \
                 standing in."
            ));
        }
        let p = PathBuf::from(dir);
        let missing = missing_markers(&p);
        if !missing.is_empty() {
            return Err(root_refusal(&p, &format!("{ROOT_OVERRIDE}={}", p.display()), &missing));
        }
        return Ok(p);
    }

    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| {
            format!("cannot run git to derive the tree to operate on, from {}: {e}", cwd.display())
        })?;
    if !out.status.success() {
        return Err(format!(
            "{} is not inside a git repository, so the tree to operate on cannot be \
             derived. Stand in the tree you mean to operate on, or set {ROOT_OVERRIDE} to \
             its {HARNESS_SUBDIR} directory.",
            cwd.display()
        ));
    }
    let top = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    let p = top.join(HARNESS_SUBDIR);
    let missing = missing_markers(&p);
    if !missing.is_empty() {
        return Err(root_refusal(
            &p,
            &format!("the working directory {} (git toplevel {})", cwd.display(), top.display()),
            &missing,
        ));
    }
    Ok(p)
}

/// The root for a tool that MAY have been told one by a parent.
///
/// Told: verify it exactly as a derived one is verified, and use it — that is what makes
/// the child unable to disagree with the parent about which tree the run belongs to.
/// Not told (a human running the tool by hand): fall through to [`resolve_harness_root`],
/// which derives from the invoking tree and refuses rather than guess. There is no third
/// branch, and in particular no branch that reaches [`BUILT_FROM`].
///
/// A told root and a set [`ROOT_OVERRIDE`] that name different trees are refused rather
/// than ranked. The two spellings exist to say which tree a run belongs to; when they
/// disagree, one of them is about to be ignored, and being ignored quietly is the shape
/// of the defect this module exists to prevent.
pub fn resolve_passed_root(
    passed: Option<&OsStr>,
    cwd: &Path,
    override_dir: Option<&OsStr>,
) -> Result<PathBuf, String> {
    let Some(passed) = passed else {
        return resolve_harness_root(cwd, override_dir);
    };
    if passed.is_empty() {
        return Err(format!(
            "{ROOT_FLAG} was given an empty directory. Give it the {HARNESS_SUBDIR} \
             directory of the tree to operate on, or omit it to derive the tree you are \
             standing in."
        ));
    }
    let p = PathBuf::from(passed);
    let missing = missing_markers(&p);
    if !missing.is_empty() {
        return Err(root_refusal(&p, &format!("{ROOT_FLAG} {}", p.display()), &missing));
    }
    if let Some(env_dir) = override_dir {
        if !env_dir.is_empty() && PathBuf::from(env_dir) != p {
            return Err(format!(
                "{ROOT_FLAG} and {ROOT_OVERRIDE} name different trees, so one of them is \
                 about to be ignored.\n  \
                 {ROOT_FLAG}:    {}\n  \
                 {ROOT_OVERRIDE}: {}\n\
                 Which tree this run belongs to is exactly the question this tool refuses \
                 to answer by guessing. Unset {ROOT_OVERRIDE}, or make it match.",
                p.display(),
                PathBuf::from(env_dir).display(),
            ));
        }
    }
    Ok(p)
}

/// The arguments a parent appends so a child operates on the SAME root it resolved.
///
/// Every child invocation goes through here, so there is one spelling of the handover and
/// a gate can hold every call site to it.
pub fn root_args(root: &Path) -> [std::ffi::OsString; 2] {
    [std::ffi::OsString::from(ROOT_FLAG), root.as_os_str().to_os_string()]
}

/// How a `repin` child is spawned: program, arguments, and the directory to spawn it in.
/// Separated from the spawn so the handover of the root can be gated — against the real
/// child binary — without running a freeze.
pub struct RepinInvocation {
    pub program: std::ffi::OsString,
    pub args: Vec<std::ffi::OsString>,
    pub cwd: PathBuf,
}

/// Build the `repin` invocation for `root`, TELLING the child which tree to operate on.
///
/// Neither spawning shape can choose its own tree. `cargo run` inherits the environment
/// (CARGO_TARGET_DIR / SIGIL_EMIT / AEON_DIR) and is instant when the binary is already
/// built, but it is free to hand back an artifact COMPILED IN A DIFFERENT CHECKOUT: with
/// one shared target directory a `repin` first built in another worktree is reused here
/// without recompiling at all, so a child left to resolve its own paths would write
/// `src/pins.rs` into that other tree while the parent wrote goldens into this one.
/// `REPIN_BIN` names a prebuilt binary and skips the rebuild outright, which is the same
/// exposure without the maybe. So the root is passed explicitly on BOTH paths, and a
/// child too old to understand [`ROOT_FLAG`] refuses the argument instead of running
/// against the wrong tree.
///
/// `cwd` is a second layer and not the mechanism: the child is spawned in the root so a
/// derivation would agree, but the gates deliberately spawn it elsewhere, because a
/// handover that only works from the right directory is not a handover.
pub fn repin_invocation(root: &Path, repin_bin: Option<std::ffi::OsString>) -> RepinInvocation {
    let mut args: Vec<std::ffi::OsString> = match &repin_bin {
        Some(_) => Vec::new(),
        None => ["run", "-q", "-p", "sigil-harness", "--bin", "repin", "--"]
            .iter()
            .map(std::ffi::OsString::from)
            .collect(),
    };
    args.extend(root_args(root));
    RepinInvocation {
        program: repin_bin.unwrap_or_else(|| std::ffi::OsString::from("cargo")),
        args,
        cwd: root.to_path_buf(),
    }
}

/// Say, UNASKED and in words, which tree this binary was built from and which one it is
/// about to operate on.
///
/// Unasked because a diagnostic that must be requested is useless against a stale binary:
/// nobody runs a check for a problem they do not know they have, and this class presents
/// as something else entirely — a broken worktree, an unknown argument — never as "the
/// binary is older than the question you are asking it".
///
/// In words because two paths side by side are raw material for a verdict, not a verdict.
/// A reader who has to notice a difference will not notice it, and the one occasion it
/// matters is the occasion they are busy.
///
/// The comparison is textual. Two spellings of the same directory would be reported as a
/// difference, which is the harmless direction: a false alarm costs a second look, and a
/// false all-clear is the failure being prevented.
pub fn announce_root(tool: &str, root: &Path) {
    let operating = root.display().to_string();
    // Reached as displayed text, never as a path — see the constant's contract.
    let built = format!("{BUILT_FROM}");
    if operating == built {
        eprintln!("{tool}: built from and operating on the same tree: {operating}");
        return;
    }
    eprintln!("{tool}: THIS BINARY WAS BUILT FROM A DIFFERENT TREE THAN IT IS OPERATING ON.");
    eprintln!("{tool}:   built from:   {BUILT_FROM}");
    eprintln!("{tool}:   operating on: {operating}");
    eprintln!(
        "{tool}: they differ. A prebuilt binary is a snapshot of its source at link \
         time — its paths, its flags and its defaults all date from when it was built, \
         not from now. If it predates what you are about to ask it, rebuild it in the \
         tree above before trusting this run."
    );
}

#[cfg(test)]
mod root_derivation {
    use super::*;
    use std::path::PathBuf;

    /// Run a git command in `dir` and insist it worked. An absent or broken git is an
    /// UNMEASURABLE gate, so it panics: these gates are about what git resolves, and a
    /// silent pass here would be the one outcome that means nothing.
    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("git {args:?} could not run in {}: {e} — this gate measures what git resolves, so it cannot be skipped", dir.display()));
        assert!(
            out.status.success(),
            "git {args:?} failed in {}: {}",
            dir.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// A canonical, real directory to build fixtures under, so paths compare against what
    /// `git rev-parse --show-toplevel` returns without symlink noise.
    fn base(tmp: &tempfile::TempDir) -> PathBuf {
        tmp.path().canonicalize().unwrap()
    }

    /// Plant a repository whose `crates/sigil-harness` carries every marker.
    fn init_repo(root: &Path) {
        std::fs::create_dir_all(root).unwrap();
        git(root, &["init", "-q"]);
        git(root, &["config", "user.email", "gate@example.invalid"]);
        git(root, &["config", "user.name", "gate"]);
        plant_markers(&root.join(HARNESS_SUBDIR));
        git(root, &["add", HARNESS_SUBDIR]);
        git(root, &["commit", "-q", "-m", "fixture"]);
    }

    /// Every marker the constant names, and nothing else — so adding a marker to the
    /// constant extends these gates instead of quietly leaving them behind.
    fn plant_markers(harness: &Path) {
        for m in ROOT_MARKERS {
            let p = harness.join(m);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, "# fixture\n").unwrap();
        }
    }

    /// THE DEFECT, as a gate. A linked worktree must resolve to ITSELF: the freeze that
    /// prompted this ran from a dedicated worktree and wrote into the shared checkout,
    /// because the binary carried that checkout's path from link time.
    #[test]
    fn a_linked_worktree_resolves_to_itself_not_to_the_checkout_it_was_made_from() {
        let tmp = tempfile::tempdir().unwrap();
        let base = base(&tmp);
        let main = base.join("main");
        init_repo(&main);
        let wt = base.join("wt");
        git(&main, &["worktree", "add", "-q", wt.to_str().unwrap()]);

        let got = resolve_harness_root(&wt, None).expect("a worktree carrying the markers resolves");
        assert_eq!(got, wt.join(HARNESS_SUBDIR));
        assert_ne!(
            got,
            main.join(HARNESS_SUBDIR),
            "resolving to the checkout the worktree was made from is the whole defect"
        );

        // And from anywhere INSIDE it, since nobody stands at the toplevel.
        let deep = wt.join(HARNESS_SUBDIR).join("golden");
        assert_eq!(resolve_harness_root(&deep, None).unwrap(), wt.join(HARNESS_SUBDIR));
    }

    /// A tree that is not a harness root is refused BY NAME, carrying all three facts:
    /// what it resolved to, where it resolved from, and what it looked for and missed.
    #[test]
    fn a_tree_without_the_markers_is_refused_with_what_where_and_expected() {
        let tmp = tempfile::tempdir().unwrap();
        let base = base(&tmp);
        let repo = base.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"]);

        let e = resolve_harness_root(&repo, None)
            .expect_err("a repository with no harness root must refuse, not guess");
        let resolved = repo.join(HARNESS_SUBDIR);
        assert!(e.contains(&resolved.display().to_string()), "must say WHAT it resolved to: {e}");
        assert!(e.contains(&repo.display().to_string()), "must say WHERE from: {e}");
        for m in ROOT_MARKERS {
            assert!(e.contains(m), "must name the marker `{m}` it expected: {e}");
        }

        // HALF the pair is not a harness root either — that is the point of a pair, and
        // it is what separates this tree from any other one that happens to have a
        // `repin.toml`. Each marker is removed in turn, so a marker added to the constant
        // is exercised here rather than quietly trusted.
        for absent in ROOT_MARKERS {
            plant_markers(&resolved);
            std::fs::remove_file(resolved.join(absent)).unwrap();
            let e = resolve_harness_root(&repo, None)
                .expect_err(&format!("half a marker pair must refuse (missing `{absent}`)"));
            assert!(e.contains(absent), "the refusal must name the missing `{absent}`: {e}");
        }
    }

    /// The override is a way to name another tree, not a way to skip the check.
    #[test]
    fn an_override_that_does_not_verify_is_refused_and_an_empty_one_too() {
        let tmp = tempfile::tempdir().unwrap();
        let base = base(&tmp);
        let elsewhere = base.join("elsewhere/crates/sigil-harness");
        plant_markers(&elsewhere);
        let bare = base.join("bare");
        std::fs::create_dir_all(&bare).unwrap();

        // It works, from a cwd that could never have derived it.
        let got = resolve_harness_root(&bare, Some(elsewhere.as_os_str()))
            .expect("a verified override is the legitimate cross-tree invocation");
        assert_eq!(got, elsewhere);

        // Set but wrong: refused, naming the variable and every missing marker.
        let e = resolve_harness_root(&bare, Some(bare.as_os_str()))
            .expect_err("an override pointing at a non-harness tree must refuse");
        assert!(e.contains(ROOT_OVERRIDE), "the refusal must name the variable: {e}");
        assert!(e.contains(&bare.display().to_string()), "must say what it resolved to: {e}");
        for m in ROOT_MARKERS {
            assert!(e.contains(m), "must name the marker `{m}`: {e}");
        }

        // Set but empty is a mistake, not an unset: an empty path would join the markers
        // against the working directory and could verify by accident.
        let e = resolve_harness_root(&bare, Some(std::ffi::OsStr::new("")))
            .expect_err("an empty override must refuse rather than resolve relatively");
        assert!(e.contains(ROOT_OVERRIDE), "{e}");
    }

    /// NO SILENT FALLBACK. When derivation fails and no override is given, the tool
    /// refuses. An optional override that defaults to the compile-time constant would
    /// leave every existing call site on today's behaviour while advertising a fix.
    #[test]
    fn derivation_failure_refuses_instead_of_falling_back_to_the_build_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let base = base(&tmp);
        let e = resolve_harness_root(&base, None)
            .expect_err("outside any harness tree there is nothing to operate on");
        assert!(!e.is_empty());
        assert!(
            e.contains(&base.display().to_string()),
            "even the give-up must say where it was standing: {e}"
        );
    }

    /// THE SPLIT, as a function-level gate: the root a parent resolves and the root its
    /// child resolves must be the SAME tree, and the child must reach it without any help
    /// from its own surroundings.
    ///
    /// The fixture makes all three candidate answers distinct — the parent's tree, the
    /// child's working directory, and the tree the binaries were built in — so the only
    /// way to pass is to honour what was passed. A child that derived from its own cwd
    /// answers with the second; a child that fell back to its compile-time constant
    /// answers with the third.
    #[test]
    fn a_told_root_wins_over_the_childs_own_surroundings() {
        let tmp = tempfile::tempdir().unwrap();
        let base = base(&tmp);
        let parent = base.join("parent");
        init_repo(&parent);
        let elsewhere = base.join("elsewhere");
        init_repo(&elsewhere);

        let parent_root = resolve_harness_root(&parent, None).expect("the parent tree resolves");
        assert_eq!(parent_root, parent.join(HARNESS_SUBDIR));

        // Exactly the handover the parent performs, read back the way the child reads it.
        let args = root_args(&parent_root);
        assert_eq!(args[0], std::ffi::OsString::from(ROOT_FLAG));

        // The child stands in a DIFFERENT harness tree — one that would resolve perfectly
        // well on its own, so deriving instead of honouring is a silent wrong answer
        // rather than an error.
        let child = resolve_passed_root(Some(&args[1]), &elsewhere, None)
            .expect("a told root that verifies is the root");
        assert_eq!(child, parent_root, "the child must operate on the tree it was told");
        assert_ne!(
            child,
            elsewhere.join(HARNESS_SUBDIR),
            "deriving from the child's own working directory is the split this gate exists for"
        );
        // Compared as text, not as a path: the build tree is a diagnostic string here and
        // the lint below holds it to that, so even this gate may not reach it as a value.
        assert_ne!(
            child.display().to_string(),
            format!("{BUILT_FROM}"),
            "falling back to the tree the binary was built in is the split's other half"
        );
    }

    /// Told nothing, the child behaves exactly like the parent does: derive from the
    /// invoking tree, verify, refuse rather than guess.
    #[test]
    fn a_child_told_nothing_derives_from_its_own_tree_and_still_refuses_a_bad_one() {
        let tmp = tempfile::tempdir().unwrap();
        let base = base(&tmp);
        let repo = base.join("repo");
        init_repo(&repo);

        assert_eq!(
            resolve_passed_root(None, &repo, None).unwrap(),
            resolve_harness_root(&repo, None).unwrap(),
            "with nothing passed the child must resolve identically to the parent"
        );

        let bare = base.join("bare");
        std::fs::create_dir_all(&bare).unwrap();
        let e = resolve_passed_root(None, &bare, None)
            .expect_err("outside a harness tree a child with no root must refuse");
        assert!(e.contains(&bare.display().to_string()), "{e}");
    }

    /// A told root is verified, not trusted — and an empty one is a mistake, not an unset.
    #[test]
    fn a_told_root_that_does_not_verify_is_refused_by_name() {
        let tmp = tempfile::tempdir().unwrap();
        let base = base(&tmp);
        let bare = base.join("bare");
        std::fs::create_dir_all(&bare).unwrap();

        let e = resolve_passed_root(Some(bare.as_os_str()), &base, None)
            .expect_err("a told root that carries no markers must refuse");
        assert!(e.contains(ROOT_FLAG), "the refusal must name the flag: {e}");
        assert!(e.contains(&bare.display().to_string()), "must say what it resolved to: {e}");
        for m in ROOT_MARKERS {
            assert!(e.contains(m), "must name the marker `{m}`: {e}");
        }

        let e = resolve_passed_root(Some(std::ffi::OsStr::new("")), &base, None)
            .expect_err("an empty told root must refuse rather than resolve relatively");
        assert!(e.contains(ROOT_FLAG), "{e}");
    }

    /// Two spellings naming two trees is a split in waiting: refuse, naming both, rather
    /// than rank them and ignore one.
    #[test]
    fn a_told_root_disagreeing_with_the_override_is_refused_naming_both() {
        let tmp = tempfile::tempdir().unwrap();
        let base = base(&tmp);
        let told = base.join("told/crates/sigil-harness");
        let env = base.join("env/crates/sigil-harness");
        plant_markers(&told);
        plant_markers(&env);

        let e = resolve_passed_root(Some(told.as_os_str()), &base, Some(env.as_os_str()))
            .expect_err("two verified but different trees must refuse");
        assert!(e.contains(&told.display().to_string()), "must name the told tree: {e}");
        assert!(e.contains(&env.display().to_string()), "must name the override tree: {e}");
        assert!(e.contains(ROOT_FLAG) && e.contains(ROOT_OVERRIDE), "must name both spellings: {e}");

        // Agreeing is not a conflict.
        assert_eq!(
            resolve_passed_root(Some(told.as_os_str()), &base, Some(told.as_os_str())).unwrap(),
            told
        );
    }

    /// EVERY spawning shape carries the root. `REPIN_BIN` is the one that bypasses the
    /// rebuild unconditionally, so it is the one most able to be a stale binary from
    /// another tree, and it gets the root for exactly that reason.
    #[test]
    fn every_repin_spawning_shape_carries_the_root() {
        let root = PathBuf::from("/some/tree/crates/sigil-harness");
        let expected = root_args(&root);

        let cargo = repin_invocation(&root, None);
        assert_eq!(cargo.program, std::ffi::OsString::from("cargo"));
        assert!(
            cargo.args.windows(2).any(|w| w == expected),
            "the cargo shape must pass the root: {:?}",
            cargo.args
        );
        let sep = cargo
            .args
            .iter()
            .position(|a| a == "--")
            .expect("cargo needs `--` or the root reaches cargo instead of the child");
        let flag = cargo.args.iter().position(|a| a == ROOT_FLAG).unwrap();
        assert!(sep < flag, "the root must come AFTER `--`: {:?}", cargo.args);

        let prebuilt = repin_invocation(&root, Some(std::ffi::OsString::from("/prebuilt/repin")));
        assert_eq!(prebuilt.program, std::ffi::OsString::from("/prebuilt/repin"));
        assert_eq!(
            prebuilt.args,
            expected.to_vec(),
            "a prebuilt child takes the root and nothing else"
        );

        // And the child that receives either arg list resolves to the root, from a
        // working directory that has nothing to do with it.
        let tmp = tempfile::tempdir().unwrap();
        let base = base(&tmp);
        let real = base.join("real/crates/sigil-harness");
        plant_markers(&real);
        for inv in [repin_invocation(&real, None), repin_invocation(&real, Some("x".into()))] {
            let told = inv.args.iter().position(|a| a == ROOT_FLAG).unwrap() + 1;
            assert_eq!(resolve_passed_root(Some(&inv.args[told]), &base, None).unwrap(), real);
        }
    }

    /// The compile-time manifest directory survives as a DIAGNOSTIC and nothing else: it
    /// may be declared, discussed in comments, and DISPLAYED — never used as a path. A
    /// fallback would be written as a path expression, so the shape is the rule.
    ///
    /// The gate also holds the two tools that resolve roots to having no compile-time
    /// path of their own AT ALL, which is what makes "there is no silent fallback" a
    /// property of the source rather than a claim about the branches someone read.
    #[test]
    fn the_compile_time_manifest_dir_is_only_ever_displayed() {
        let src = include_str!("harness_root.rs");
        // Split so this gate does not match its own source.
        let token = concat!("BUILT_", "FROM");
        let decl = format!("const {token}");
        let capture = format!("{{{token}}}");

        assert!(
            src.matches(token).count() >= 2,
            "nothing to measure: the diagnostic constant is not in this file under the \
             name this gate knows"
        );
        assert!(
            src.contains(&capture),
            "the build tree must be PRINTED somewhere, or the diagnostic does not exist"
        );

        let offenders: Vec<String> = src
            .lines()
            .enumerate()
            .filter(|(_, l)| l.contains(token))
            .filter(|(_, l)| {
                let t = l.trim_start();
                !(t.starts_with("//") || l.contains(&decl) || l.contains(&capture))
            })
            .map(|(i, l)| format!("{}: {}", i + 1, l.trim()))
            .collect();
        assert!(
            offenders.is_empty(),
            "the build tree is reachable as a value here, which is how a fallback gets \
             written: {offenders:#?}"
        );

        // Neither tool may carry a compile-time path of its own. The macro is the only
        // way to spell one, so its absence is the whole proof.
        let macro_token = concat!("CARGO_MANIFEST", "_DIR");
        for (name, tool) in [
            ("refreeze", include_str!("bin/refreeze.rs")),
            ("repin", include_str!("bin/repin.rs")),
        ] {
            assert!(tool.contains("fn main"), "nothing to measure: {name}'s source is not here");
            let offenders: Vec<String> = tool
                .lines()
                .enumerate()
                .filter(|(_, l)| l.contains(macro_token))
                .map(|(i, l)| format!("{name} {}: {}", i + 1, l.trim()))
                .collect();
            assert!(
                offenders.is_empty(),
                "a tool that resolves a root must have no compile-time path of its own; \
                 the diagnostic constant lives in this module and is display-only: \
                 {offenders:#?}"
            );
        }
    }
}
