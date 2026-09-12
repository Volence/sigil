//! `refreeze --attest` NAMES THE LEGACY ORACLE TREE BEFORE THE SUITE RUNS, and hands it
//! to the suite explicitly.
//!
//! The strict suite `--attest` runs contains a gate that compiles against the legacy
//! oracle tree (`oracle-old`), and that gate refuses a tree nobody named. An attest that
//! left the tree to the suite met that refusal twenty minutes in and recorded it, as a
//! FAILED run, in the append-only chain.
//!
//! These rows drive the real binary against a scratch sigil tree whose chain holds and
//! whose tip awaits its strict run, so `--attest` gets as far as naming its trees. `cargo`
//! is replaced on `PATH` by a stub that records the arguments and environment it was
//! handed and exits with no result line, so a run that gets past the naming ends at the
//! tool's own "no `test result:` line" refusal. Nothing is built and nothing is ever
//! recorded; every row asserts the chain is byte-identical afterwards.
//!
//! Four directions, each asserted on wording only it produces:
//!
//!   * NOTHING NAMED: refused before the suite, by this rule's name; no stub ran, no log
//!     was stamped. The same bed with the tree named DOES reach the stub, so the absence
//!     is a refusal and not a stub that never runs;
//!   * `ORACLE_DIR` NAMED: accepted, announced as step 1, stamped into the log header with
//!     its revision, and present in the child's environment;
//!   * `EMPYREAN_SUITE_ROOT` NAMED, `ORACLE_DIR` unset: accepted at step 2, and the child
//!     receives `ORACLE_DIR=<root>/oracle-old`. This is the row that proves the handover
//!     is EXPLICIT: the tool's own environment carries no `ORACLE_DIR`, so the child can
//!     only have one because the tool set it;
//!   * `ORACLE_DIR` SET BUT WRONG beside a valid suite root: refused, and the suite root
//!     does not answer in its place.
//!
//! Runner: `cargo test -p sigil-harness --test attest_names_the_oracle_tree`, and the
//! plain workspace run. No reference tree, no strict gate, no emulator.

use sigil_harness::provenance::{self, ASL_WITNESS};
use sigil_harness::test_support::{
    AEON_DIR_VAR, ORACLE_DIR_VAR, ORACLE_LEGACY_REPO_DIR, SUITE_ROOT_MARKERS, SUITE_ROOT_VAR,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The phrase only the legacy-oracle refusal uses. No other `--attest` refusal, and not
/// the suite's own bare-run refusal, contains it.
const REFUSAL: &str = "needs a NAMED legacy oracle tree";

/// Where a run that got past the naming ends: the stub prints no result line.
const REACHED_THE_SUITE: &str = "the run produced NO `test result:` line";

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Make `dir` a CLEAN checkout of one commit holding everything in it; returns its HEAD.
fn commit_all(dir: &Path) -> String {
    git(dir, &["init", "-q"]);
    git(dir, &["add", "."]);
    git(
        dir,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "seed",
        ],
    );
    git(dir, &["rev-parse", "HEAD"])
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// A scratch sigil tree `--attest` accepts up to the point of naming its trees, an aeon
/// checkout at the revision its tip names, and the `cargo` stub.
struct Bed {
    tmp: tempfile::TempDir,
    sigil: PathBuf,
    harness: PathBuf,
    aeon: PathBuf,
    stub_dir: PathBuf,
    /// Written by the stub, only when it runs: its argv line, then its environment.
    record: PathBuf,
    /// Passed as `--log`, so the header stamp is readable at a known path.
    log: PathBuf,
}

impl Bed {
    fn plant() -> Bed {
        let tmp = tempfile::tempdir().expect("tempdir");
        let base = tmp.path().to_path_buf();

        // The aeon tree: clean, one revision, which the chain tip names.
        let aeon = base.join("aeon");
        std::fs::create_dir_all(&aeon).expect("mkdir aeon");
        std::fs::write(aeon.join("f"), "x").expect("write aeon file");
        let aeon_rev = commit_all(&aeon);

        // The sigil tree: a harness root with one golden blob and a one-entry chain whose
        // tip is that blob, derived with the same `recompute_targets` a real freeze uses.
        let sigil = base.join("sigil");
        let harness = sigil.join("crates/sigil-harness");
        let golden = harness.join("golden");
        std::fs::create_dir_all(&golden).expect("mkdir golden");
        let blob: Vec<u8> = (0..512u32).map(|n| n as u8).collect();
        std::fs::write(golden.join("alpha.bin"), &blob).expect("write blob");
        let map = BTreeMap::from([("alpha".to_string(), "alpha.bin".to_string())]);
        let ends = BTreeMap::from([("alpha".to_string(), 496usize)]);
        let targets = provenance::recompute_targets(&golden, &map, &ends).expect("recompute");
        let chain =
            provenance::render_entry("root", ASL_WITNESS, &aeon_rev, "the scratch root", &targets);
        std::fs::write(golden.join("provenance.toml"), &chain).expect("write chain");
        std::fs::write(harness.join("repin.toml"), "").expect("write repin marker");

        // One strict-gated test, so the census `--attest` takes before naming its trees
        // has a population to find. The guard's name is assembled at run time so that
        // this file declares no strict-gate site of its own to the real census.
        let guard = ["strict", "_gate"].concat();
        let tests = sigil.join("crates/fixture/tests");
        std::fs::create_dir_all(&tests).expect("mkdir fixture tests");
        std::fs::write(
            tests.join("gate.rs"),
            format!(
                "#[test]\nfn a_strict_gated_body() {{\n    if !{guard}() {{\n        return;\n    }}\n}}\n"
            ),
        )
        .expect("write fixture gate");
        commit_all(&sigil);

        // POSITIVE CONTROLS. A bed `--attest` refused for another reason would make every
        // row below measure that reason instead, so the bed's state is measured with the
        // tool's own predicates rather than assumed.
        let parsed = provenance::parse(&chain).expect("the scratch chain must parse");
        let errs = provenance::check(&golden, &parsed);
        assert!(errs.is_empty(), "the scratch chain must hold: {errs:?}");
        assert!(
            parsed.tip().expect("tip").strict.is_none(),
            "the scratch tip must await its strict run, or --attest refuses at step (0)"
        );
        let census = sigil_harness::strict_census::census(&sigil.join("crates"))
            .expect("the scratch census must be derivable");
        assert_eq!(census.sites.len(), 1, "the scratch tree declares exactly one strict gate");

        // The `cargo` stub: records what it was handed, prints nothing, fails.
        let stub_dir = base.join("bin");
        std::fs::create_dir_all(&stub_dir).expect("mkdir stub dir");
        let record = base.join("cargo-was-run");
        let stub = stub_dir.join("cargo");
        std::fs::write(
            &stub,
            format!(
                "#!/bin/bash\n{{ printf 'ARGV'; printf ' %s' \"$@\"; printf '\\n'; env; }} > '{}'\nexit 1\n",
                record.display()
            ),
        )
        .expect("write stub");
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&stub).expect("stat stub").permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&stub, perm).expect("chmod stub");

        Bed { log: base.join("attest.log"), tmp, sigil, harness, aeon, stub_dir, record }
    }

    /// Run `refreeze --attest` on the bed. Neither tree-naming variable for the legacy
    /// oracle is inherited from the environment this test runs in (a landing run exports
    /// `ORACLE_DIR`); a row that wants one passes it in `env`.
    fn attest(&self, env: &[(&str, &Path)]) -> Output {
        let path =
            format!("{}:{}", self.stub_dir.display(), std::env::var("PATH").unwrap_or_default());
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_refreeze"));
        cmd.arg("--attest")
            .arg("--log")
            .arg(&self.log)
            .current_dir(&self.sigil)
            .env("SIGIL_HARNESS_ROOT", &self.harness)
            .env(AEON_DIR_VAR, &self.aeon)
            .env("CARGO_TARGET_DIR", self.tmp.path().join("target"))
            .env("PATH", path)
            .env_remove(ORACLE_DIR_VAR)
            .env_remove(SUITE_ROOT_VAR);
        for (k, v) in env {
            cmd.env(k, v);
        }
        cmd.output().expect("run refreeze")
    }

    fn chain(&self) -> Vec<u8> {
        std::fs::read(self.harness.join("golden/provenance.toml")).expect("read scratch chain")
    }

    /// A directory that IS a suite root by the resolver's own marker set, holding an
    /// `oracle-old` directory.
    fn suite_root(&self) -> PathBuf {
        let suite = self.tmp.path().join("suite");
        for d in SUITE_ROOT_MARKERS.iter().chain([&ORACLE_LEGACY_REPO_DIR]) {
            std::fs::create_dir_all(suite.join(d)).expect("mkdir suite member");
        }
        suite
    }

    /// The stub's argv line and environment, or `None` when the stub never ran.
    fn child(&self) -> Option<(String, BTreeMap<String, String>)> {
        let text = std::fs::read_to_string(&self.record).ok()?;
        let mut lines = text.lines();
        let argv = lines.next().unwrap_or("").to_string();
        let env = lines
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Some((argv, env))
    }
}

/// The command the tool runs, as the stub saw it. Asserted so a row that reads the
/// stub's record knows the record is the SUITE's invocation and not some other cargo call.
fn assert_is_the_suite(argv: &str) {
    assert!(
        argv.starts_with("ARGV test --release --workspace"),
        "the stub's record must be the strict suite's invocation: {argv}"
    );
}

#[test]
fn an_attest_that_names_no_legacy_oracle_tree_is_refused_before_the_suite() {
    let bed = Bed::plant();
    let before = bed.chain();

    let out = bed.attest(&[]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(2), "--attest must refuse: {err}");
    assert!(err.contains(REFUSAL), "the refusal must be the legacy-oracle rule's own: {err}");
    let set = format!("{ORACLE_DIR_VAR}=<{ORACLE_LEGACY_REPO_DIR} checkout>");
    assert!(
        err.contains(&set) && err.contains(SUITE_ROOT_VAR),
        "the refusal must give the line to set, `{set}`, and the suite-root alternative: {err}"
    );
    // Which unnamed shape this machine produced, stated rather than assumed: a machine
    // whose layout derives a suite root declines that derivation (step 3), and one that
    // derives nothing reports step 4. Both are refusals; the pure row in refreeze.rs
    // reaches the step-3 branch on every machine.
    let derived = err.contains("SUITE_PATHS step 3");
    let nothing = err.contains("no oracle-old checkout could be resolved");
    assert!(derived || nothing, "the resolver's own answer must be carried: {err}");
    eprintln!(
        "unnamed shape on this machine: {}",
        if derived { "step 3 derived, declined" } else { "step 4, nothing resolved" }
    );
    assert!(
        !err.contains(REACHED_THE_SUITE),
        "the run must stop before the suite, not at the suite's own refusal: {err}"
    );
    assert!(bed.child().is_none(), "no suite may start before the tree is named");
    assert!(!bed.log.exists(), "no log may be stamped for a run that did not start");
    assert_eq!(bed.chain(), before, "a refusal must leave provenance.toml byte-identical");

    // THE CONTROL that makes the absence above mean something: the same bed with the tree
    // named reaches the stub.
    let named = bed.tmp.path().join("oracle-named");
    std::fs::create_dir_all(&named).expect("mkdir named");
    let out = bed.attest(&[(ORACLE_DIR_VAR, &named)]);
    assert!(
        bed.child().is_some(),
        "control: with the tree named, the same bed must reach the suite stub, or the \
         absence asserted above proves nothing:\n{}",
        stderr(&out)
    );
}

#[test]
fn a_named_oracle_dir_is_accepted_stamped_and_handed_to_the_suite() {
    let bed = Bed::plant();
    let before = bed.chain();
    let named = bed.tmp.path().join("oracle-named");
    std::fs::create_dir_all(&named).expect("mkdir named");
    std::fs::write(named.join("f"), "x").expect("write named file");
    let rev = commit_all(&named);

    let out = bed.attest(&[(ORACLE_DIR_VAR, &named)]);
    let err = stderr(&out);
    assert!(!err.contains(REFUSAL), "a named tree must not be refused: {err}");
    assert!(err.contains(REACHED_THE_SUITE), "the run must reach the suite: {err}");
    let announced =
        format!("reference-tree: {} (SUITE_PATHS step 1, named by {ORACLE_DIR_VAR})", named.display());
    assert!(err.contains(&announced), "the run must say which step named the tree: {err}");

    let log = std::fs::read_to_string(&bed.log).expect("the log is stamped before the suite");
    let stamp = format!("# ORACLE_DIR     {} (step 1, named by {ORACLE_DIR_VAR})", named.display());
    assert!(log.contains(&stamp), "the log header must name the tree and the step: {log}");
    let head = format!("# oracle HEAD    {rev} (clean)");
    assert!(log.contains(&head), "the log header must name the tree's revision: {log}");

    let (argv, env) = bed.child().expect("the suite stub must have run");
    assert_is_the_suite(&argv);
    assert_eq!(
        env.get(ORACLE_DIR_VAR).map(String::as_str),
        Some(named.to_str().expect("utf-8 path")),
        "the suite must be handed the tree that was named"
    );
    assert_eq!(bed.chain(), before, "nothing may be recorded");
}

#[test]
fn a_suite_root_names_the_tree_and_the_suite_is_handed_it_explicitly() {
    let bed = Bed::plant();
    let before = bed.chain();
    let suite = bed.suite_root();
    let want = suite.join(ORACLE_LEGACY_REPO_DIR);

    let out = bed.attest(&[(SUITE_ROOT_VAR, &suite)]);
    let err = stderr(&out);
    assert!(!err.contains(REFUSAL), "a tree named at step 2 must not be refused: {err}");
    assert!(err.contains(REACHED_THE_SUITE), "the run must reach the suite: {err}");
    let announced =
        format!("reference-tree: {} (SUITE_PATHS step 2, named by {SUITE_ROOT_VAR})", want.display());
    assert!(err.contains(&announced), "the run must say step 2 named the tree: {err}");

    let log = std::fs::read_to_string(&bed.log).expect("the log is stamped before the suite");
    let stamp = format!("# ORACLE_DIR     {} (step 2, named by {SUITE_ROOT_VAR})", want.display());
    assert!(log.contains(&stamp), "the log header must name the tree and the step: {log}");

    // THE EXPLICITNESS PROOF. The tool was run with no `ORACLE_DIR` at all, so a value in
    // the child's environment can only be one the tool set.
    let (argv, env) = bed.child().expect("the suite stub must have run");
    assert_is_the_suite(&argv);
    assert_eq!(
        env.get(ORACLE_DIR_VAR).map(String::as_str),
        Some(want.to_str().expect("utf-8 path")),
        "the suite must be handed {ORACLE_DIR_VAR} explicitly, the tree step 2 resolved, \
         not left to resolve it for itself: child environment carried {:?}",
        env.get(ORACLE_DIR_VAR)
    );
    assert_eq!(bed.chain(), before, "nothing may be recorded");
}

#[test]
fn an_oracle_dir_naming_no_directory_is_refused_and_the_suite_root_does_not_answer_instead() {
    let bed = Bed::plant();
    let before = bed.chain();
    let suite = bed.suite_root();
    let wrong = bed.tmp.path().join("no-such-oracle");

    let out = bed.attest(&[(ORACLE_DIR_VAR, &wrong), (SUITE_ROOT_VAR, &suite)]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(2), "--attest must refuse: {err}");
    assert!(err.contains(REFUSAL), "the refusal must be the legacy-oracle rule's own: {err}");
    let why = format!("{ORACLE_DIR_VAR}={} does not name a directory", wrong.display());
    assert!(err.contains(&why), "the refusal must say what was wrong with the value: {err}");
    assert!(bed.child().is_none(), "the suite root must not answer in the variable's place");
    assert!(!bed.log.exists(), "no log may be stamped for a run that did not start");
    assert_eq!(bed.chain(), before, "a refusal must leave provenance.toml byte-identical");
}
