//! THE SUITE-PATHS PRECEDENCE IS THE CONTRACT'S, AND A WRONG VARIABLE STOPS AT ITS OWN STEP.
//!
//! `empyrean` `contract/SUITE_PATHS.md` fixes one precedence for every resolver in the
//! suite: (1) the explicit checkout variable `AEON_DIR`; (2) `EMPYREAN_SUITE_ROOT` joined
//! with the repo's directory name; (3) derivation from the calling repo's own location;
//! (4) refuse, naming what was looked for and where. A variable that is **set but wrong**
//! is a hard error at its own step, *not* a null that lets the next step run.
//!
//! That last clause is the whole point and the only one a casual implementation gets
//! wrong. A fall-through answers with a tree the operator did not ask for and reports
//! success — the same silent-green class the reference-tree work exists to close, arriving
//! one level down in the resolver instead of in a gate.
//!
//! ## Why every assertion runs in a CHILD PROCESS
//!
//! The property is about the ENVIRONMENT, and libtest runs a binary's tests in parallel
//! threads of one process: `set_var` from one test is visible to every other, so an
//! in-process environment test is a race that passes on ordering. Each case below is
//! therefore a child of this binary started with an environment built from scratch, and
//! the parent refuses to report green on any child it cannot account for — a child that
//! never started exits non-zero and prints nothing, which is indistinguishable from a
//! child that ran and found nothing unless the parent checks.
//!
//! ## Where the expectations come from
//!
//! Nothing here is copied from a pin or from one measurement.
//!
//!   * the step-3 expectation is derived by a walk that is INDEPENDENT of the resolver's
//!     own mechanism — up from this crate's manifest directory to the first ancestor
//!     holding every `SUITE_ROOT_MARKERS` entry — so agreeing with the resolver means two
//!     different derivations agree, not that one derivation was transcribed;
//!   * the `--show-toplevel` arm compares the resolver's answer against what that command
//!     would have produced. From a git worktree the two differ, which is why the contract
//!     names `--git-common-dir` and forbids `--show-toplevel`; when they do NOT differ
//!     (the run is in a plain checkout) the arm says it could not be measured rather than
//!     passing on an absent distinction;
//!   * step 1 and step 2's fixtures are directories this test creates, so no assertion
//!     depends on the contents of any real checkout.

use std::path::{Path, PathBuf};
use std::process::Command;

use sigil_harness::test_support::{
    aeon_checkout, unnamed_default_tree, AEON_DIR_VAR, AEON_REPO_DIR, SUITE_ROOT_MARKERS,
    SUITE_ROOT_VAR,
};

/// Selects the child body. Absent, this binary is the parent.
const CHILD_VAR: &str = "SIGIL_SUITE_PATHS_CHILD";

/// A directory this process created that no concurrent run can collide with, REMOVED AFTER
/// — including on a panic, which is the case that matters.
///
/// `contract/SUITE_PATHS.md` asks the step-3 bed's temporary worktree to be removed after,
/// and the same discipline is owed by every fixture here. Measured rather than assumed: an
/// earlier version of this file swept its directories with a `remove_dir_all` at the end of
/// the test, which a failing assertion never reaches — 68 directories survived this
/// parcel's own red-first runs, two of them beds carrying a git repository. A `Drop` runs on
/// unwind, so the sweep happens on the path where it was previously skipped.
///
/// Nothing is lost by removing a bed on failure: every assertion here quotes the paths it is
/// about into its own message, so the evidence is in the panic text and not on disk.
struct Scratch {
    path: PathBuf,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

impl Scratch {
    fn path(&self) -> &Path {
        &self.path
    }
}

fn scratch(tag: &str) -> Scratch {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is after the epoch")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("sigil-suite-paths-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&path).expect("create the scratch directory");
    Scratch { path }
}

/// The suite root found WITHOUT the resolver's mechanism: a marker walk up from this
/// crate's manifest directory. The contract names this as the alternative derivation, so
/// it is a genuinely independent answer to the same question rather than a transcription.
fn walked_suite_root() -> Option<PathBuf> {
    let mut here: Option<&Path> = Some(Path::new(env!("CARGO_MANIFEST_DIR")));
    while let Some(dir) = here {
        if SUITE_ROOT_MARKERS.iter().all(|m| dir.join(m).is_dir()) {
            return Some(dir.to_path_buf());
        }
        here = dir.parent();
    }
    None
}

// ── The child bodies. Each prints ONE machine-readable RESULT line. ─────────────

fn print_result(r: Result<sigil_harness::test_support::ResolvedCheckout, String>) {
    match r {
        Ok(c) => println!("RESULT ok step={} path={}", c.step.number(), c.path.display()),
        Err(e) => println!("RESULT err {e}"),
    }
}

fn child(mode: &str) {
    match mode {
        "resolve" => print_result(aeon_checkout()),
        "unnamed" => print_result(unnamed_default_tree()),
        other => panic!("UNMEASURABLE: unknown {CHILD_VAR} mode `{other}`"),
    }
}

// ── The parent. ────────────────────────────────────────────────────────────────

/// One child run: `mode`, plus the environment entries it should have and only those.
struct Case {
    mode: &'static str,
    /// `(name, Some(value))` sets, `(name, None)` removes.
    env: Vec<(&'static str, Option<String>)>,
}

fn run(case: &Case) -> String {
    let exe = std::env::current_exe().expect("the test binary knows its own path");
    let mut cmd = Command::new(&exe);
    cmd.arg("--nocapture").arg("--test-threads=1").env(CHILD_VAR, case.mode);
    // Every variable the resolver consults is removed first, so a case's environment is
    // exactly what it declares and never what the parent happened to inherit.
    cmd.env_remove(AEON_DIR_VAR).env_remove(SUITE_ROOT_VAR);
    for (k, v) in &case.env {
        match v {
            Some(val) => cmd.env(k, val),
            None => cmd.env_remove(k),
        };
    }
    let out = cmd.output().unwrap_or_else(|e| {
        panic!("UNMEASURABLE: could not run {} in `{}` mode: {e}", exe.display(), case.mode)
    });
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "the `{}` child failed ({}). Its output:\n{text}",
        case.mode,
        out.status
    );
    // libtest writes `test <name> ... ` without a newline before the body runs, so the
    // child's own line is a SUFFIX of a libtest line rather than a line of its own.
    let line = text
        .lines()
        .find_map(|l| l.find("RESULT ").map(|i| &l[i..]))
        .unwrap_or_else(|| {
            panic!(
                "UNMEASURABLE: the `{}` child exited 0 without a RESULT line, so it cannot be \
                 established that the resolver ran at all. A missing answer is not a passing \
                 one.\n{text}",
                case.mode
            )
        })
        .to_string();
    line
}

fn ok_case(mode: &'static str, env: Vec<(&'static str, Option<String>)>) -> (u8, PathBuf) {
    let line = run(&Case { mode, env });
    let rest = line.strip_prefix("RESULT ok step=").unwrap_or_else(|| {
        panic!("expected a resolved answer, got: {line}")
    });
    let (step, path) = rest.split_once(" path=").expect("RESULT ok carries a step and a path");
    (step.parse().expect("the step is a number"), PathBuf::from(path))
}

fn err_case(mode: &'static str, env: Vec<(&'static str, Option<String>)>) -> String {
    let line = run(&Case { mode, env });
    line.strip_prefix("RESULT err ")
        .unwrap_or_else(|| {
            panic!(
                "expected a REFUSAL and the resolver ANSWERED: {line}. A variable that is set but \
                 wrong must stop at its own step; an answer here means the resolver went on to a \
                 later step and returned a tree nobody asked for, while reporting success."
            )
        })
        .to_string()
}

/// A directory that IS a suite root by the marker rule, built here so no assertion in
/// this file depends on any real checkout's contents.
fn fake_suite_root(tag: &str) -> Scratch {
    let root = scratch(tag);
    for m in SUITE_ROOT_MARKERS {
        std::fs::create_dir_all(root.path().join(m)).expect("create a marker directory");
    }
    root
}

#[test]
fn the_resolver_follows_the_contract_precedence() {
    if let Ok(mode) = std::env::var(CHILD_VAR) {
        child(&mode);
        return;
    }

    let s = |p: &Path| Some(p.display().to_string());

    // ── STEP 1: the explicit checkout variable wins, ahead of a valid suite root.
    let named_dir = scratch("named");
    let named = named_dir.path().to_path_buf();
    let root_a_dir = fake_suite_root("root-a");
    let root_a = root_a_dir.path().to_path_buf();
    let (step, path) = ok_case(
        "resolve",
        vec![(AEON_DIR_VAR, s(&named)), (SUITE_ROOT_VAR, s(&root_a))],
    );
    assert_eq!(step, 1, "AEON_DIR names a directory, so step 1 must answer; got step {step}");
    assert_eq!(path, named, "step 1 must answer with the directory AEON_DIR names");

    // ── STEP 1 SET BUT WRONG: a hard error, and it must NOT fall through to a suite
    // root that would have answered. This is the discriminating case: with a valid
    // EMPYREAN_SUITE_ROOT present, a fall-through resolver returns a perfectly good
    // path and reports success while measuring a tree nobody asked for.
    let absent = named.join("this-path-does-not-exist");
    let err = err_case(
        "resolve",
        vec![(AEON_DIR_VAR, s(&absent)), (SUITE_ROOT_VAR, s(&root_a))],
    );
    assert!(
        err.contains(AEON_DIR_VAR) && err.contains(&absent.display().to_string()),
        "the refusal must name the variable and the path it was set to, so the fix is readable \
         from the message; got: {err}"
    );
    assert!(
        !err.contains(&root_a.join(AEON_REPO_DIR).display().to_string()),
        "a set-but-wrong AEON_DIR must stop at step 1. This refusal names the step-2 answer, so \
         the resolver went on to consult EMPYREAN_SUITE_ROOT, and a resolver that consults it \
         after a wrong AEON_DIR is one step from returning it. Got: {err}"
    );

    // ── STEP 2: the suite root answers when the checkout variable does not.
    let root_b_dir = fake_suite_root("root-b");
    let root_b = root_b_dir.path().to_path_buf();
    let (step, path) = ok_case("resolve", vec![(SUITE_ROOT_VAR, s(&root_b))]);
    assert_eq!(step, 2, "with AEON_DIR unset and a valid suite root, step 2 must answer");
    assert_eq!(
        path,
        root_b.join(AEON_REPO_DIR),
        "step 2 answers with the suite root joined with the repo's directory name"
    );

    // ── STEP 2 SET BUT WRONG: a directory that is not a suite root is a hard error, and
    // must not fall through to the derivation — which on this box WOULD have answered.
    let not_a_root_dir = scratch("not-a-root");
    let not_a_root = not_a_root_dir.path().to_path_buf();
    let err = err_case("resolve", vec![(SUITE_ROOT_VAR, s(&not_a_root))]);
    assert!(
        err.contains(SUITE_ROOT_VAR) && err.contains(&not_a_root.display().to_string()),
        "the refusal must name the variable and its value; got: {err}"
    );
    for m in SUITE_ROOT_MARKERS {
        assert!(
            err.contains(m),
            "the refusal must say what a suite root has to hold, or the reader cannot act on it: \
             `{m}` is missing from: {err}"
        );
    }

    // ── STEP 3: derivation, and it must agree with a derivation that is not its own.
    let walked = walked_suite_root().expect(
        "UNMEASURABLE: no ancestor of this crate holds the suite markers, so the step-3 \
         expectation cannot be established independently of the resolver",
    );
    let (step, path) = ok_case("resolve", vec![]);
    assert_eq!(step, 3, "with neither variable set, step 3 must answer");
    assert_eq!(
        path,
        walked.join(AEON_REPO_DIR),
        "the resolver's git-based derivation and an independent marker walk must reach the same \
         suite root; they disagree, so at least one is answering about the wrong tree"
    );

    // ── THE STEP-3 PROOF, FROM A LINKED WORKTREE THIS TEST BUILDS. See
    // `the_step_3_derivation_is_proven_from_a_linked_worktree` below; it is a separate row
    // so its own failure names the property rather than arriving as one more assertion in
    // a long precedence walk.

    // ── STEP 4: refuse, naming everything consulted. Forced by taking `git` off PATH, so
    // the derivation genuinely cannot run — no test hook in the resolver's own path.
    let err = err_case("resolve", vec![("PATH", Some("/nonexistent".to_string()))]);
    for name in [AEON_DIR_VAR, SUITE_ROOT_VAR] {
        assert!(
            err.contains(name),
            "the step-4 refusal must name every variable it consulted, or the reader cannot tell \
             what to set: `{name}` is missing from: {err}"
        );
    }
    assert!(
        err.contains("git rev-parse --git-common-dir"),
        "the step-4 refusal must say why the derivation did not answer; got: {err}"
    );

    // ── THE UNNAMED DEFAULT skips step 1 by construction: it is what a run resolves to
    // when nobody names a tree, so a value in AEON_DIR must not reach it.
    let (step, path) = ok_case("unnamed", vec![(AEON_DIR_VAR, s(&named))]);
    assert_eq!(
        step, 3,
        "unnamed_default_tree answers the question `what does a run resolve to when nobody names \
         a tree`, a set AEON_DIR must not answer it"
    );
    assert_ne!(path, named, "the unnamed default must not be the tree AEON_DIR names");
    assert_eq!(path, walked.join(AEON_REPO_DIR), "the unnamed default is step 3's answer here");
}

/// THE STEP-3 PROOF, MADE FROM A LINKED WORKTREE THIS TEST BUILDS ITSELF.
///
/// `contract/SUITE_PATHS.md`, "What a resolver owes its reader", amended 2026-09-02 from
/// aurora's O68 which they found on their own merged resolver:
///
/// > **The step-3 proof runs from a linked worktree, or says in the run's own output that
/// > it did not.** The property step 3 is written for, `--git-common-dir` answering where
/// > `--show-toplevel` answers wrongly, is only observable from a linked worktree; in the
/// > main checkout the two agree, so a test asserting it there proves nothing, and a test
/// > that skips there is honest but never runs where the suite normally runs.
///
/// This row's first version had exactly the shape the clause forbids: it asked whether the
/// TEST PROCESS happened to be running from a worktree, asserted the property if so, and
/// printed `NOT MEASURED` otherwise. Every sigil agent runs in a worktree, so it passed for
/// whoever wrote it and would have gone quiet the moment the suite ran from
/// `/home/volence/sonic_hacks/sigil` — where the landing run and the nightly lane run it. A
/// green log and an absent run are the same artifact.
///
/// So the bed is built here and the assertion is the same wherever `cargo test` is invoked
/// from:
///
/// ```text
/// <scratch>/suite/            <- a suite root by the marker rule
///   aeon/                     <- the sibling the resolver must reach
///   empyrean/
///   repo/                     <- a real git repo, standing in for this checkout
///     nested/wt/              <- a LINKED worktree, `git worktree add`
/// ```
///
/// **The worktree is NESTED INSIDE the repo, and that is the load-bearing detail.** From a
/// worktree that happens to sit beside the suite root, `--show-toplevel` plus a sibling join
/// lands on the right answer by accident and a test built on that bed passes for the wrong
/// reason. Nested, the wrong method resolves under `nested/` and finds nothing — so the two
/// methods give different answers and the assertion has something to bite on. (The same
/// shape the concurrent scripts lane measured for its own resolver; two halves of one
/// contract should not disagree about what proves it.)
///
/// If the bed cannot be built — no `git`, no writable scratch, a git too old for
/// `worktree add` — this row PRINTS why and does not assert. That is the clause's own
/// escape, and it is a printed line rather than an `ignored` for the reason the clause
/// gives.
#[test]
fn the_step_3_derivation_is_proven_from_a_linked_worktree() {
    if std::env::var_os(CHILD_VAR).is_some() {
        return; // the children run the resolver, not this row
    }
    let bed = scratch("worktree-bed");

    let suite = bed.path().join("suite");
    let repo = suite.join("repo");
    for d in [&suite.join(AEON_REPO_DIR), &suite.join("empyrean"), &repo] {
        std::fs::create_dir_all(d).expect("create the bed");
    }

    // A real repository with a commit: `git worktree add` needs a ref to check out.
    let git = |args: &[&str], cwd: &std::path::Path| -> Result<String, String> {
        let out = Command::new("git")
            .args(args)
            // A bed must not inherit the developer's identity, hooks or templates.
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_AUTHOR_NAME", "sigil-test")
            .env("GIT_AUTHOR_EMAIL", "sigil-test@invalid")
            .env("GIT_COMMITTER_NAME", "sigil-test")
            .env("GIT_COMMITTER_EMAIL", "sigil-test@invalid")
            .current_dir(cwd)
            .output()
            .map_err(|e| format!("`git {}` could not run: {e}", args.join(" ")))?;
        if !out.status.success() {
            return Err(format!(
                "`git {}` exited {}: {}",
                args.join(" "),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };

    let built = (|| -> Result<PathBuf, String> {
        git(&["init", "-q", "-b", "main", "."], &repo)?;
        std::fs::write(repo.join("seed"), b"bed\n").map_err(|e| format!("write seed: {e}"))?;
        git(&["add", "seed"], &repo)?;
        git(&["commit", "-q", "-m", "bed"], &repo)?;
        // NESTED, not beside: see the doc comment. `--detach` so the bed leaves no branch
        // behind either — the same shape the concurrent scripts lane planted for its own
        // resolver, matched rather than reinvented.
        let wt = repo.join("nested").join("wt");
        std::fs::create_dir_all(repo.join("nested")).map_err(|e| format!("mkdir nested: {e}"))?;
        git(&["worktree", "add", "-q", "--detach", "nested/wt"], &repo)?;
        Ok(wt)
    })();

    let wt = match built {
        Ok(wt) => wt,
        Err(why) => {
            // The clause's escape, printed rather than silent.
            println!(
                "NOT MEASURED: could not build the linked-worktree bed, so the step-3 property \
                 was not exercised in this run, {why}. Everything else in this file still ran; \
                 what is missing is the one assertion that separates `--git-common-dir` from \
                 `--show-toplevel`."
            );
            return; // `Scratch`'s Drop sweeps the bed, worktree registration and all
        }
    };

    // THE CONTROL, and without it the assertion below could pass on a bed where the two
    // methods happen to agree — which is the defect the amendment is about.
    let toplevel = git(&["rev-parse", "--show-toplevel"], &wt)
        .map(PathBuf::from)
        .expect("the bed's worktree answers --show-toplevel");
    let wrong_root = toplevel.parent().map(|p| p.to_path_buf());
    assert_ne!(
        wrong_root.as_deref(),
        Some(suite.as_path()),
        "UNMEASURABLE: on this bed `--show-toplevel`'s parent IS the suite root, so the wrong \
         method would give the right answer and passing proves nothing. The worktree must be \
         nested inside the repo, not beside the suite root."
    );

    // THE PROPERTY: the resolver's own step-3 mechanism, run from inside that worktree,
    // reaches the suite root that the repo hangs off — not the one `--show-toplevel` implies.
    let derived = sigil_harness::test_support::derive_suite_root_from(&wt);
    let derived = derived.unwrap_or_else(|e| {
        panic!(
            "step 3 could not derive a suite root from the linked worktree {}, {e}. This is the \
             shape every sigil agent runs in, so a derivation that fails here fails in ordinary \
             use.",
            wt.display()
        )
    });
    // REPORT THE PAIR *AND* THE RETURNED SOURCE, per `contract/SUITE_PATHS.md` paragraph 4
    // (empyrean `8dfb07f`, from aurora finding it on their own merged O68 work). The pair
    // alone is the trap: a run that never executed the resolver from the bed still prints a
    // pair in which every path is correct — the wrong method wrong at the temp worktree, the
    // resolver answering a true suite root — because the pair only establishes that the bed
    // is a place where the wrong method is wrong. It says nothing about whether the resolver
    // was standing there. The RETURNED SOURCE is what says that, and the assertion below is
    // what makes it load-bearing rather than decorative.
    //
    // Measured here, not inherited: with that assertion neutered and the pair left intact,
    // this row goes GREEN and prints a fully correct pair. With the resolver made to ignore
    // the anchor it is handed, it goes RED with `left: /home/volence/sonic_hacks` — the main
    // checkout — which is the failure paragraph 4 requires regardless of the pair.
    println!(
        "step-3 bed: wrong method -> {:?}; bed's suite root -> {}; RETURNED SOURCE -> {}",
        wrong_root,
        suite.display(),
        derived.display()
    );
    assert_eq!(
        derived, suite,
        "step 3 derived the wrong suite root from a LINKED, NESTED worktree. `--show-toplevel` \
         there answers {}, the worktree, whose parent is {:?}; only `--git-common-dir`, which \
         answers the main checkout's `.git` from a worktree and from the checkout alike, reaches \
         the repository this code belongs to.",
        toplevel.display(),
        wrong_root
    );

    // And the same mechanism is what PRODUCTION uses: `derived_suite_root()` is this
    // function called with the crate's compile-time manifest directory. Asserted rather
    // than assumed, because a helper proven in isolation and a production path that calls
    // something else is the classic way a gate ends up measuring nothing.
    let live = sigil_harness::test_support::derive_suite_root_from(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )));
    let walked = walked_suite_root();

    // WHICH SHAPE THIS RUN EXERCISED, said out loud. The production anchor is a crate
    // subdirectory, so git answers `../../.git` when this binary was compiled in a plain
    // checkout and an absolute path when it was compiled in a linked worktree. Both must
    // work, and only one of them is exercised HERE — the other is covered unconditionally
    // by `step_3_survives_every_shape_git_rev_parse_can_answer`.
    //
    // Printing it is the cheap fix for how the third shape survived: this row was green in
    // every agent worktree for as long as it existed, and its output never said that the
    // configuration it had checked was the easy one.
    let raw = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "<git did not answer>".to_string());
    println!(
        "production anchor {} -> git --git-common-dir answers `{raw}` ({}); the other shapes are \
         covered by step_3_survives_every_shape_git_rev_parse_can_answer",
        env!("CARGO_MANIFEST_DIR"),
        if raw.starts_with('/') { "absolute, a linked worktree" } else { "RELATIVE, a plain checkout" }
    );

    match (live, walked) {
        (Ok(l), Some(w)) => assert_eq!(
            l, w,
            "the mechanism proven on the bed, applied to this crate's own location, disagrees \
             with an independent marker walk, so the bed proved a function the production path \
             does not behave like"
        ),
        (Err(e), _) => panic!("step 3 cannot derive from this crate's own location: {e}"),
        (_, None) => panic!(
            "UNMEASURABLE: no ancestor of this crate holds the suite markers, so the production \
             half of this row has no independent expectation to check against"
        ),
    }

    ambient_worktree_check(&walked_suite_root());
}

/// THE SECONDARY CHECK: the same property against the REAL DEPLOYED ANCHOR.
///
/// The bed above is the row that must always run and always mean something — it is
/// invariant to where `cargo test` was invoked from. This one is the opposite trade and is
/// kept for what only it can reach: `derived_suite_root()`'s production anchor,
/// `env!("CARGO_MANIFEST_DIR")`, in whatever checkout this binary was actually compiled in.
/// The bed proves the walk; this proves the anchor the walk is deployed behind.
///
/// It can therefore only assert when the ambient run happens to be in a linked worktree,
/// which is exactly why it is SECONDARY. When it cannot, it prints why — and the printed
/// line says which of the two checks was skipped, so a reader is not left thinking the
/// step-3 property went unmeasured when the bed measured it.
fn ambient_worktree_check(walked: &Option<PathBuf>) {
    let Some(walked) = walked else {
        println!(
            "NOT MEASURED (secondary only): no ancestor of this crate holds the suite markers, so \
             the deployed anchor has no independent expectation here. The bed above still proved \
             the step-3 walk."
        );
        return;
    };
    let toplevel = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()));
    let Some(top) = toplevel else {
        println!(
            "NOT MEASURED (secondary only): `git rev-parse --show-toplevel` did not run against \
             this crate's own location. The bed above still proved the step-3 walk."
        );
        return;
    };
    if top.parent() == Some(walked.as_path()) {
        println!(
            "NOT MEASURED (secondary only): this binary was compiled in a plain checkout, where \
             `--show-toplevel` and `--git-common-dir` agree, so the deployed anchor cannot \
             exercise the distinction here. The bed above proved it on a bed that can, which is \
             why that row and not this one is the contract's requirement."
        );
        return;
    }
    // Compiled inside a linked worktree — the shape every sigil agent runs in. The
    // deployed derivation must reach the checkout's suite root, not the worktree's parent.
    let live = sigil_harness::test_support::derive_suite_root_from(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .expect("the deployed anchor derives a suite root");
    assert_eq!(
        &live, walked,
        "compiled inside a linked worktree, the deployed anchor derived the wrong suite root. \
         `--show-toplevel` answers {} there, whose parent is {:?}, only `--git-common-dir` \
         reaches the checkout this code belongs to.",
        top.display(),
        top.parent()
    );
}

/// EVERY SHAPE `git rev-parse --git-common-dir` CAN ANSWER, and step 3 must survive all of
/// them.
///
/// ## The bug this row exists for
///
/// The first version of `derive_suite_root_from` carried a doc comment enumerating TWO
/// shapes — "relative (`.git`) when git answers from a checkout's own root, absolute from a
/// worktree" — and there are three. Anchored at a SUBDIRECTORY of a plain checkout, git
/// answers `../../.git`: relative, and carrying `..`. `Path::parent()` trims components
/// lexically without canonicalising, so two `parent()` calls on
/// `<crate>/../../.git` yield `<crate>/..`, whose `aeon/` sits one directory
/// away from where the walk was looking. Step 3 then refused, and every row downstream of
/// it failed.
///
/// `CARGO_MANIFEST_DIR` is ALWAYS a crate subdirectory, so the missing shape is the one
/// production uses — in the main checkout, which is where the landing run and the nightly
/// lane invoke this suite.
///
/// ## Why the existing rows did not catch it, and why this one is shaped as it is
///
/// `the_step_3_derivation_is_proven_from_a_linked_worktree` plants a bed and anchors at the
/// worktree's ROOT, where git answers absolutely — the shape that worked. And
/// `ambient_worktree_check` does anchor at `CARGO_MANIFEST_DIR`, which would have caught
/// this — but only when compiled in a plain checkout, and every sigil agent compiles in a
/// linked worktree. Between them the suite proved step 3 in the two configurations where it
/// worked and in none where it did not. **The contract clause's own warning, inverted: it
/// worried about a row that proves nothing outside a worktree; this was a resolver that
/// worked only inside one.**
///
/// So this row does not pick a configuration. It ENUMERATES them — the four anchors a
/// caller can plausibly hand the resolver — and requires one answer from all four. A shape
/// nobody thought of is then a failing row rather than a sentence missing from a doc
/// comment, which is exactly how the original was lost.
///
/// A control asserts the shapes are genuinely DIFFERENT before the equality is asserted:
/// if git answered identically everywhere, agreement would prove nothing.
#[test]
fn step_3_survives_every_shape_git_rev_parse_can_answer() {
    let bed = scratch("shapes");
    let suite = bed.path().join("suite");
    let repo = suite.join("repo");
    // The crate subdirectory at the real relative depth: `crates/<crate>`, which is what
    // `CARGO_MANIFEST_DIR` names.
    let crate_dir = repo.join("crates").join("sigil-harness");
    for d in [&suite.join(AEON_REPO_DIR), &suite.join("empyrean"), &crate_dir] {
        std::fs::create_dir_all(d).expect("create the bed");
    }

    let git = |args: &[&str], cwd: &std::path::Path| -> Result<String, String> {
        let out = Command::new("git")
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_AUTHOR_NAME", "sigil-test")
            .env("GIT_AUTHOR_EMAIL", "sigil-test@invalid")
            .env("GIT_COMMITTER_NAME", "sigil-test")
            .env("GIT_COMMITTER_EMAIL", "sigil-test@invalid")
            .current_dir(cwd)
            .output()
            .map_err(|e| format!("`git {}` could not run: {e}", args.join(" ")))?;
        if !out.status.success() {
            return Err(format!(
                "`git {}` exited {}: {}",
                args.join(" "),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };

    let built = (|| -> Result<PathBuf, String> {
        git(&["init", "-q", "-b", "main", "."], &repo)?;
        std::fs::write(repo.join("seed"), b"bed\n").map_err(|e| format!("write seed: {e}"))?;
        git(&["add", "-A"], &repo)?;
        git(&["commit", "-q", "-m", "bed"], &repo)?;
        let wt = repo.join("nested").join("wt");
        std::fs::create_dir_all(repo.join("nested")).map_err(|e| format!("mkdir nested: {e}"))?;
        git(&["worktree", "add", "-q", "--detach", "nested/wt"], &repo)?;
        std::fs::create_dir_all(wt.join("crates").join("sigil-harness"))
            .map_err(|e| format!("mkdir in worktree: {e}"))?;
        Ok(wt)
    })();

    let wt = match built {
        Err(why) => {
            println!(
                "NOT MEASURED: could not build the shapes bed, so step 3 was exercised against \
                 none of the four anchors in this run, {why}."
            );
            return;
        }
        Ok(wt) => wt,
    };

    // THE FOUR ANCHORS. `CARGO_MANIFEST_DIR` is a crate subdirectory, so rows 2 and 4 are
    // the production ones; rows 1 and 3 are what a caller standing at a checkout root gets.
    let anchors: [(&str, PathBuf); 4] = [
        ("plain checkout ROOT", repo.clone()),
        ("plain checkout CRATE SUBDIR (what CARGO_MANIFEST_DIR names)", crate_dir.clone()),
        ("linked worktree ROOT", wt.clone()),
        ("linked worktree CRATE SUBDIR", wt.join("crates").join("sigil-harness")),
    ];

    // The raw shapes, reported so the run's output carries what git actually said. This is
    // the evidence a reader needs when a future git changes one of them.
    let mut shapes = Vec::new();
    for (name, anchor) in &anchors {
        let raw = git(&["rev-parse", "--git-common-dir"], anchor)
            .unwrap_or_else(|e| panic!("UNMEASURABLE: git could not answer at {name}: {e}"));
        println!("shape [{name}] -> {raw}");
        shapes.push(raw);
    }

    // THE CONTROL. If git answered identically at every anchor, the equality below would
    // hold for a resolver that ignored the difference, and this row would prove nothing —
    // which is precisely how the missing third shape survived review.
    let distinct: std::collections::BTreeSet<&String> = shapes.iter().collect();
    assert!(
        distinct.len() > 1,
        "UNMEASURABLE: `git rev-parse --git-common-dir` answered the same string at all four \
         anchors ({:?}), so this bed cannot tell a resolver that handles every shape from one \
         that handles a single shape.",
        shapes
    );

    // THE PROPERTY: one suite root from all four, whatever git said.
    for ((name, anchor), raw) in anchors.iter().zip(&shapes) {
        let got = sigil_harness::test_support::derive_suite_root_from(anchor);
        let got = got.unwrap_or_else(|e| {
            panic!(
                "step 3 REFUSED at the `{name}` anchor.\n  anchor: {}\n  git answered: {raw}\n  \
                 error: {e}\nEvery anchor a caller can hand this resolver must reach the same \
                 suite root; `CARGO_MANIFEST_DIR` is a crate subdirectory, so a shape that fails \
                 here fails in production.",
                anchor.display()
            )
        });
        assert_eq!(
            got,
            suite,
            "step 3 derived the WRONG suite root at the `{name}` anchor.\n  anchor: {}\n  git \
             answered: {raw}\n  derived: {}\n  expected: {}\nA relative answer carrying `..` is \
             trimmed LEXICALLY by Path::parent(), not canonicalised, so the walk lands beside \
             the tree it was looking for.",
            anchor.display(),
            got.display(),
            suite.display()
        );
    }
}
