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

/// A directory this process created that no concurrent run can collide with.
fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is after the epoch")
        .as_nanos();
    let p = std::env::temp_dir().join(format!("sigil-suite-paths-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&p).expect("create the scratch directory");
    p
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
fn fake_suite_root(tag: &str) -> PathBuf {
    let root = scratch(tag);
    for m in SUITE_ROOT_MARKERS {
        std::fs::create_dir_all(root.join(m)).expect("create a marker directory");
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
    let named = scratch("named");
    let root_a = fake_suite_root("root-a");
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
         the resolver went on to consult EMPYREAN_SUITE_ROOT — and a resolver that consults it \
         after a wrong AEON_DIR is one step from returning it. Got: {err}"
    );

    // ── STEP 2: the suite root answers when the checkout variable does not.
    let root_b = fake_suite_root("root-b");
    let (step, path) = ok_case("resolve", vec![(SUITE_ROOT_VAR, s(&root_b))]);
    assert_eq!(step, 2, "with AEON_DIR unset and a valid suite root, step 2 must answer");
    assert_eq!(
        path,
        root_b.join(AEON_REPO_DIR),
        "step 2 answers with the suite root joined with the repo's directory name"
    );

    // ── STEP 2 SET BUT WRONG: a directory that is not a suite root is a hard error, and
    // must not fall through to the derivation — which on this box WOULD have answered.
    let not_a_root = scratch("not-a-root");
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

    // ── `--show-toplevel` LIES FROM A WORKTREE, which is why the contract forbids it.
    let toplevel = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()));
    match toplevel {
        Some(top) if top.parent() != Some(walked.as_path()) => {
            // A worktree: `--show-toplevel` answers the worktree, whose parent is not the
            // suite root. A resolver built on it derives a confidently wrong answer.
            let wrong = top.parent().map(|p| p.join(AEON_REPO_DIR));
            assert_ne!(
                Some(path.clone()),
                wrong,
                "the resolver answered with the path `--show-toplevel` would have produced. From \
                 a worktree that command names the worktree, not the checkout, so the derivation \
                 must use `--git-common-dir`."
            );
        }
        Some(_) => println!(
            "NOT MEASURED: this run is not in a git worktree, so `--show-toplevel` and \
             `--git-common-dir` agree here and the distinction the contract draws cannot be \
             exercised. The assertion above still holds the derivation to an independent walk."
        ),
        None => println!(
            "NOT MEASURED: `git rev-parse --show-toplevel` did not run, so the worktree \
             distinction could not be exercised in this environment."
        ),
    }

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
         a tree` — a set AEON_DIR must not answer it"
    );
    assert_ne!(path, named, "the unnamed default must not be the tree AEON_DIR names");
    assert_eq!(path, walked.join(AEON_REPO_DIR), "the unnamed default is step 3's answer here");
}
