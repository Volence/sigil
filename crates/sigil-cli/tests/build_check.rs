//! `sigil build --check`: decide every guard of a game against final placement,
//! write nothing.
//!
//! The mode runs the chained build's resolve and stops before the link, so it can
//! only be driven where that resolve can run: over an aeon tree, with its map,
//! its AS residual and its registry. A self-contained `.emp` program cannot stand
//! in for that (the census arithmetic is gated tree-free in
//! `sigil-harness/tests/check_only_census.rs`); these gates drive the real CLI over
//! the reference tree on the demo shape, the per-commit path aeon asked for, through
//! `sigil build --project <tree>/games/<game> --shape <row>` (G0b Task 11). A build scans
//! only its engine root and its own game's root (G0b Task 9), so the committed poison,
//! sonic4's, is driven on the sonic4 `release` shape, and the unreachable-guard control
//! plants its own pure-comptime module in demo's root of a copy of the tree (no committed
//! demo or engine module is both unreachable from demo and ensures-only with guards that
//! hold).
//!
//! Asserted, each with its control:
//!   - exit 0 and the census line on a clean tree; exit 1 with the guard's own
//!     `[Error]` line when a guard fails (`--extra-entry` on a committed poison);
//!   - the banner that says what a green check does not prove;
//!   - no ROM after a check, against a full build that does write one at the
//!     path a check refuses;
//!   - a guard outside the target's closure is reported unreachable and left out
//!     of the census, and pulling it in with `--extra-entry` raises the count.
//!
//! Reference tree: `AEON_DIR` (the house pattern; `SIGIL_STRICT_GATE` hard-fails a
//! missing tree, `SIGIL_ALLOW_PARTIAL` skips).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon \
//!   cargo test --release -p sigil-cli --test build_check
//! ```

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The committed poison `extra_entry.rs` also drives, with the fragment aeon pins
/// for it: exactly one guard, one fragment unique to it.
const POISON_NESTED_PATH: &str = "games/sonic4/test/poison/poison_band_nested.emp";
const POISON_NESTED_FRAGMENT: &str = "two bands are live on CRAM entry";

/// A pure-comptime module (consts + `ensure`s) outside the DEMO closure whose guards
/// hold: the unreachable-guard control, planted in demo's root of a copy of the tree.
const OUTSIDE_DEMO_CLOSURE: &str = "games.demo.check_unreachable_probe";
const OUTSIDE_DEMO_CLOSURE_PATH: &str = "games/demo/check_unreachable_probe.emp";
const OUTSIDE_DEMO_CLOSURE_SRC: &str = "module games.demo.check_unreachable_probe

const PROBE = 2
ensure(PROBE == 2, \"build_check probe: holds\")
ensure(PROBE > 1, \"build_check probe: holds too\")
";

/// The banner fragment every check run must print before it runs.
const BANNER: &str = "a green check proves nothing about region budget or overlap, image bounds, \
                      the checksum or the contract closure gate, and is not a statement that the game builds";

fn aeon_dir() -> Option<PathBuf> {
    sigil_harness::test_support::reference_tree_for_shape(&sigil_harness::shapes::Shape::new("demo", "release"))
}

// Every run reads the same tree; serialize so one run's output is its own.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `sigil build --project <tree>/games/demo --shape release <args>` from `cwd`, returning the
/// process result, stdout and stderr.
fn sigil_demo(aeon: &Path, cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> (Output, String, String) {
    sigil_game(aeon, "demo", cwd, args, env)
}

/// Run `sigil build --project <tree>/games/<game> --shape release <args>` from `cwd`.
/// AEON_DIR is removed: the tree may be a copy, and the build observes its checkout.
fn sigil_game(aeon: &Path, game: &str, cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> (Output, String, String) {
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.current_dir(cwd).args(["build", "--project"]).arg(aeon.join("games").join(game)).args(["--shape", "release"]).args(args);
    cmd.env_remove("AEON_DIR");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("run sigil build");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (out, stdout, stderr)
}

/// A fresh empty directory for one case, so "nothing was written" is a
/// statement about a directory this test alone populates.
fn fresh_dir(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sigil_build_check_{case}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn entries(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .expect("read temp dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// Parse the census line: `(ensure verdicts, LinkAsserts decided, inapplicable)`.
fn census(stdout: &str) -> (usize, usize, usize) {
    let line = stdout
        .lines()
        .find(|l| l.starts_with("checked: demo release: ") || l.starts_with("checked: sonic4 release: "))
        .unwrap_or_else(|| panic!("no census line in stdout:\n{stdout}"));
    let nums: Vec<usize> = line
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().expect("count"))
        .collect();
    assert!(
        line.contains(" ensure verdict(s) at comptime, ")
            && line.contains(" LinkAssert(s) decided at link, ")
            && line.contains(" LinkAssert(s) inapplicable (extern not defined in this link, allowlisted)"),
        "{line}"
    );
    assert_eq!(nums.len(), 3, "{line}");
    (nums[0], nums[1], nums[2])
}

/// Clean tree: exit 0, the banner on stderr, the census on stdout with both
/// families non-empty (demo carries comptime guards and extern guards), and the
/// working directory untouched. The control for "untouched": the full build over
/// the same tree writes a ROM at the path it is given, and a check given that
/// same path refuses it at parse time and still writes nothing.
#[test]
fn a_clean_check_decides_both_families_and_writes_nothing() {
    let Some(aeon) = aeon_dir() else { return };
    let dir = fresh_dir("clean");

    let (out, stdout, stderr) = sigil_demo(&aeon, &dir, &["--check"], &[]);
    assert!(out.status.success(), "check failed:\n{stdout}{stderr}");
    assert!(stderr.contains(BANNER), "banner missing:\n{stderr}");
    assert!(
        stderr.lines().next().is_some_and(|l| l.starts_with("check: demo release: ")),
        "the banner is the first stderr line:\n{stderr}"
    );
    let (comptime, decided, inapplicable) = census(&stdout);
    assert!(comptime > 0, "demo decided no comptime guard: {stdout}");
    assert!(decided > 0, "demo decided no LinkAssert: {stdout}");
    assert_eq!(inapplicable, 0, "demo's allowlist is empty: {stdout}");
    assert_eq!(entries(&dir), Vec::<String>::new(), "a check wrote into its cwd");

    // Positive control: the full build writes the ROM where it is told to.
    let rom = dir.join("demo.bin");
    let (out, stdout, stderr) = sigil_demo(&aeon, &dir, &["-o", rom.to_str().unwrap()], &[]);
    assert!(out.status.success(), "full build failed:\n{stdout}{stderr}");
    assert!(stdout.contains("Built ") && stdout.contains(" shape `release`"), "{stdout}");
    let len = std::fs::metadata(&rom).expect("the full build wrote demo.bin").len();
    assert!(len > 0, "empty ROM");
    // A full build also writes the build-data export beside the ROM (G0a); the control
    // removes both so the refusal below is checked against an empty directory.
    let sidecar = dir.join("demo.build.json");
    assert!(sidecar.exists(), "the full build wrote no build.json beside {}", rom.display());
    std::fs::remove_file(&rom).expect("remove the control ROM");
    std::fs::remove_file(&sidecar).expect("remove the control sidecar");

    // The same destination beside --check is refused, and nothing appears.
    let (out, stdout, stderr) = sigil_demo(&aeon, &dir, &["--check", "-o", rom.to_str().unwrap()], &[]);
    assert_eq!(out.status.code(), Some(2), "a usage error:\n{stdout}{stderr}");
    assert!(stderr.contains("--check decides guards without writing a ROM"), "{stderr}");
    assert!(!rom.exists(), "a refused check wrote {}", rom.display());
    assert_eq!(entries(&dir), Vec::<String>::new());

    let _ = std::fs::remove_dir_all(&dir);
}

/// A failing guard: exit 1, the guard's own message on the `[Error]` surface at
/// its `path:line:col`, exactly one `[Error]`, and no census line (a run that
/// stopped on a guard reports the guard, not a count that reads as green).
#[test]
fn a_failing_guard_fails_the_check_with_its_rendered_line() {
    let Some(aeon) = aeon_dir() else { return };
    let dir = fresh_dir("failing");
    let (out, stdout, stderr) = sigil_game(&aeon, "sonic4", &dir, &["--check", "--extra-entry", POISON_NESTED_PATH], &[]);
    assert_eq!(out.status.code(), Some(1), "output:\n{stdout}{stderr}");
    assert!(stderr.contains("error: native check (sonic4 release): "), "{stderr}");
    assert!(stderr.contains(POISON_NESTED_FRAGMENT), "the guard's own message:\n{stderr}");
    let all = format!("{stdout}{stderr}");
    assert_eq!(all.matches("[Error]").count(), 1, "one firing guard, one [Error]:\n{all}");
    // The guard that emits this message lives in the raster DSL the poison drives,
    // so the located line names an `.emp` under the tree at `path:line:col`, with
    // the `[Error]` token, rendered by the same path the full build uses.
    let prefix = format!("  {}/", aeon.display());
    assert!(
        stderr.lines().any(|l| l.starts_with(&prefix)
            && l.contains(".emp:")
            && l.contains(": [Error] ")
            && l.contains(POISON_NESTED_FRAGMENT)),
        "the guard is rendered as `path:line:col: [Error] msg`:\n{stderr}"
    );
    assert!(!stdout.contains("checked: "), "no census after a failed check:\n{stdout}");
    assert_eq!(entries(&dir), Vec::<String>::new(), "a failed check wrote into its cwd");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A guard outside the target's closure is reported, not decided: the planted probe
/// module is unreachable from the demo entry, its guards are named
/// `[module.unreachable]` and are absent from the census. Pulling it in with
/// `--extra-entry` removes the row and raises the comptime count, which is the
/// proof that the count line reads the evaluator's verdicts and not the tree.
#[test]
fn an_unreachable_guard_is_reported_not_decided() {
    let Some(reference) = aeon_dir() else { return };
    let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_check_tree_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&copy);
    std::fs::create_dir_all(&copy).unwrap();
    let aeon = copy.join("tree");
    sigil_harness::test_support::copy_tree(&reference, &aeon);
    std::fs::write(aeon.join(OUTSIDE_DEMO_CLOSURE_PATH), OUTSIDE_DEMO_CLOSURE_SRC).unwrap();
    let dir = fresh_dir("unreachable");
    let full = [("SIGIL_WARNINGS", "full")];
    let row = format!("{}:", aeon.join(OUTSIDE_DEMO_CLOSURE_PATH).display());
    let tree_prefix = format!("{}/", aeon.display());
    let file = format!("/{}:", Path::new(OUTSIDE_DEMO_CLOSURE_PATH).file_name().unwrap().to_str().unwrap());
    let is_row = |l: &str| l.starts_with(&row) && l.starts_with(&tree_prefix) && l.contains(&file);

    let (out, stdout, stderr) = sigil_demo(&aeon, &dir, &["--check"], &full);
    assert!(out.status.success(), "{stdout}{stderr}");
    let unreachable: Vec<&str> = stderr
        .lines()
        .filter(|l| is_row(l) && l.contains("[module.unreachable]") && l.contains(OUTSIDE_DEMO_CLOSURE))
        .collect();
    assert_eq!(unreachable.len(), 1, "the module is reported unreachable once:\n{stderr}");
    assert!(unreachable[0].contains("`ensure` guard(s) are never evaluated for this target"), "{}", unreachable[0]);
    let (without, decided_without, _) = census(&stdout);

    let (out, stdout, stderr) =
        sigil_demo(&aeon, &dir, &["--check", "--extra-entry", OUTSIDE_DEMO_CLOSURE], &full);
    assert!(out.status.success(), "{stdout}{stderr}");
    assert!(
        !stderr.lines().any(|l| is_row(l) && l.contains("[module.unreachable]")),
        "the row must be gone once the module is in the closure:\n{stderr}"
    );
    let (with, decided_with, _) = census(&stdout);
    assert!(with > without, "pulling the module in must raise the comptime count: {without} -> {with}");
    assert_eq!(decided_with, decided_without, "a pure-comptime module defers no LinkAssert");
    assert_eq!(entries(&dir), Vec::<String>::new());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&copy);
}

/// The committed fixture this file names still resolves, so a renamed poison reds here
/// rather than inside someone else's landing (the unreachable probe is planted, not
/// committed).
#[test]
fn every_aeon_fixture_this_file_names_still_resolves() {
    let Some(aeon) = aeon_dir() else { return };
    let path = POISON_NESTED_PATH;
    assert!(aeon.join(path).is_file(), "`{path}` is named by this file but is not under {}", aeon.display());
}
