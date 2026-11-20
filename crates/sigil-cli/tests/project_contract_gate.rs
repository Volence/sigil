//! `sigil build --project` runs the contract closure gate (G0b Task 8 step 0, QUEUE
//! G0B-TASK5-NEW-PATH-GAPS): before the cutover the game-as-data driver built with no
//! closure gate at all, so a `build.sh` arm switched to it would have shipped ROMs nothing
//! contract-checked.
//!
//! The gate is the profile-driven driver's; what differs is the module set and where its
//! frozen rows come from. On `--project` the closure analyses the engine root and the game's
//! own root only (G0b Task 9a, `Project::closure_roots`), and its rows are read from the
//! descriptions: the `baseline.toml` the engine description names and the one the game
//! description names (`sigil_build::baseline`). Another game's sources are not analysed and
//! its files are not read. A game that does not declare a constant the engine description's
//! `required_consts` names is refused by name before the gate runs.
//!
//! A self-contained two-game fixture tree (engine + game `g` is built, `h` is the other game),
//! whose baselines are sized to its own corpus, so the EXIT CODE carries the verdict here:
//! the clean tree passes the gate (the build then stops at the next stage, the fixture has
//! no engine constants, which is past the gate and not this file's business). Each red
//! case plants one thing in a fresh copy and reads both the exit and the refusal text.
//!
//! Runner: `cargo test -p sigil-cli` (the landing bar's full suite).

use std::path::Path;
use std::process::{Command, Output};

const ENGINE_TOML: &str = r#"format = 1
helpers = []
ram_helpers = []
struct_twins = []
entry_seeds = ["engine.game_contract", "engine.ram"]
terminus = "engine.epilogue"
error_handler = "engine.debug.error_handler"
release_fault = "engine.system.release_fault"
required_labels = []
required_consts = ["REQ_CONST"]
baseline = "baseline.toml"
constants = "constants.emp"
types = "types.emp"
error_handler_blob = "x"
error_handler_blob_len = 1
convsym = "convsym"
fault_island_heads = []
ram_floor = 0xF00000
[defines]
HOTKEYS = 0
[sound]
blob_bank_ids = []
blob_start = "a"
blob_end = "b"
dac_head = "c"
dac_blip_bank = "d"
dac_shared_bank = "e"
"#;

/// `EngCaller` holds `d0` across `Callee`, which clobbers it: one `[call.live-clobbered]`
/// row, on an ENGINE proc.
const ENGINE_EMP: &str = "module engine.eng
pub proc Callee () clobbers(d0) {
    moveq   #0, d0
    rts
}
pub proc EngCaller () clobbers(d0-d1) {
    moveq   #5, d0
    jbsr    Callee
    move.w  d0, d1
    rts
}
";

/// The other game's own proc, with the same D1c shape: a row on a proc in `h`'s sources.
const H_EMP: &str = "module games.h.hmod
use engine.eng
pub proc HCaller () clobbers(d0-d1) {
    moveq   #5, d0
    jbsr    Callee
    move.w  d0, d1
    rts
}
";

/// A proc that writes `d2` and declares only `d0`: a `[proc.clobber-undeclared]` firing,
/// zero-firing by contract.
const VIOLATION: &str = "pub proc Bad () clobbers(d0) {
    moveq   #0, d2
    rts
}
";

const ENGINE_BASELINE: &str = "format = 1\nd1c = [[\"EngCaller\", \"Callee\", \"d0\"]]\n";
/// `h`'s file: its own proc's row, and a row on an ENGINE proc (the shape of demo's
/// `Parallax_Update` row: a fact about building `h`, which a `g` shape must not inherit).
const H_BASELINE: &str = "format = 1\nd1c = [[\"HCaller\", \"Callee\", \"d0\"], [\"EngCaller\", \"Callee\", \"d0\"]]\n";
const G_BASELINE: &str = "format = 1\n";
/// `g`'s declaration of the constant the fixture engine requires.
const G_EMP: &str = "module games.g.gconst\npub const REQ_CONST = -1\n";

fn game_toml(id: &str) -> String {
    format!(
        r#"format = 1
id = "{id}"
name = "{id}"
require_one_text = false
appendix_floor = 4096
map = "map.toml"
baseline = "baseline.toml"
engine = {{ suite_path = "aeon", root = "engine" }}
sources = {{ root = ".", residual_as = "game_root.asm" }}
entry = {{ game = "games.{id}.game", ram = "games.{id}.ram" }}
[shapes.release]
features = {{ debug = false, crash_report = true, sound = false }}
"#
    )
}

/// The clean fixture tree under `root`, with `edit(path, text)` applied to each file first.
fn tree(root: &Path, edit: impl Fn(&str, String) -> String) {
    let files: [(&str, String); 10] = [
        ("engine/engine.toml", ENGINE_TOML.to_string()),
        ("engine/baseline.toml", ENGINE_BASELINE.to_string()),
        ("engine/eng.emp", ENGINE_EMP.to_string()),
        ("games/g/game.toml", game_toml("g")),
        ("games/g/map.toml", String::new()),
        ("games/g/baseline.toml", G_BASELINE.to_string()),
        ("games/g/gconst.emp", G_EMP.to_string()),
        ("games/h/game.toml", game_toml("h")),
        ("games/h/baseline.toml", H_BASELINE.to_string()),
        ("games/h/hmod.emp", H_EMP.to_string()),
    ];
    for (rel, text) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, edit(rel, text)).unwrap();
    }
    std::fs::write(root.join("games/h/map.toml"), "").unwrap();
}

fn sigil(root: &Path, args: &[&str], contracts_off: bool) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.current_dir(root).arg("build").args(args).env_remove("AEON_DIR");
    if contracts_off {
        cmd.env("SIGIL_CONTRACTS", "0");
    } else {
        cmd.env_remove("SIGIL_CONTRACTS");
    }
    cmd.output().expect("run sigil build")
}

fn project_build(root: &Path) -> (String, Output) {
    let out = sigil(root, &["--project", "games/g", "--shape", "release", "-o", "g.bin"], false);
    (String::from_utf8_lossy(&out.stderr).into_owned(), out)
}

/// Two modules: the engine's and `g`'s. `h`'s module is scanned but not analysed.
const PASSED: &str = "contract closure gate: passed (g release: 2 module(s) analysed;";
const FAILED: &str = "error: contract closure gate FAILED.";

/// CONTROL: the clean tree passes the gate on `--project`. It passes only if `h`'s module
/// is not analysed (else `HCaller` is NEW) and `h`'s file is not read (else `EngCaller` is
/// wanted twice and GONE once, and `HCaller` is GONE). The pass line names exactly the two
/// baseline files read.
#[test]
fn the_clean_tree_passes_the_project_gate() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |_, t| t);
    let (err, _) = project_build(tmp.path());
    assert!(err.contains(PASSED), "the clean fixture must pass the gate, got:\n{err}");
    assert!(!err.contains(FAILED), "got:\n{err}");
    let home = tmp.path().canonicalize().unwrap();
    let files = format!("{} or {})", home.join("engine/baseline.toml").display(), home.join("games/g/baseline.toml").display());
    assert!(err.contains(&format!("; baseline {files}")), "the pass line names exactly the engine's and g's files, got:\n{err}");
    assert!(!err.contains("games/h/"), "nothing of h is read, got:\n{err}");
}

/// A planted closure violation: `--project` refuses it, with the same located refusal line
/// the profile-driven driver prints for the same file.
#[test]
fn a_planted_closure_violation_is_refused_on_project_as_on_aeon() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |rel, t| if rel == "engine/eng.emp" { format!("{t}{VIOLATION}") } else { t });
    let (err, out) = project_build(tmp.path());
    assert_eq!(out.status.code(), Some(1), "got:\n{err}");
    assert!(err.contains(FAILED), "got:\n{err}");
    assert!(!err.contains("contract closure gate: passed"), "got:\n{err}");
    let line = err
        .lines()
        .find(|l| l.contains("[proc.clobber-undeclared] `Bad` destroys `d2`"))
        .unwrap_or_else(|| panic!("no located refusal for `Bad`, got:\n{err}"))
        .to_string();
    // The profile-driven driver over the same tree (it reads `games/sonic4/map.toml`).
    std::fs::create_dir_all(tmp.path().join("games/sonic4")).unwrap();
    std::fs::write(tmp.path().join("games/sonic4/map.toml"), "").unwrap();
    let abs = tmp.path().canonicalize().unwrap();
    let legacy = sigil(tmp.path(), &["--aeon", abs.to_str().unwrap(), "--game", "sonic4", "-o", "s4.bin"], false);
    let lerr = String::from_utf8_lossy(&legacy.stderr);
    assert!(lerr.contains(FAILED), "got:\n{lerr}");
    assert!(lerr.lines().any(|l| l == line), "--aeon must print the same refusal line\n{line}\ngot:\n{lerr}");
}

/// Baseline drift in the game's own file: add a row for a proc that does not fire and it is
/// GONE, refused with the adjudication message naming the description files.
#[test]
fn a_drifted_row_in_the_games_file_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |rel, t| if rel == "games/g/baseline.toml" { "format = 1\nd1c = [[\"HCaller\", \"Callee\", \"d0\"]]\n".into() } else { t });
    let (err, out) = project_build(tmp.path());
    assert_eq!(out.status.code(), Some(1), "got:\n{err}");
    assert!(err.contains("[call.live-clobbered] (D1c) moved against the frozen baseline."), "got:\n{err}");
    assert!(err.contains("\"HCaller @ Callee :: d0 (got 0, want 1)\""), "got:\n{err}");
    assert!(err.contains("games/g/baseline.toml"), "the message names the description files, got:\n{err}");
    assert!(err.contains(FAILED), "got:\n{err}");
}

/// The other game is not this build (G0b Task 9a): a closure violation planted in `h`'s
/// sources, with `h`'s baseline file deleted, does not reach `g`'s gate, which passes;
/// the profile-driven driver, whose gate analyses the whole scan, refuses the same tree.
#[test]
fn another_games_sources_and_files_are_not_this_games_gate() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |rel, t| if rel == "games/h/hmod.emp" { format!("{t}{VIOLATION}") } else { t });
    std::fs::remove_file(tmp.path().join("games/h/baseline.toml")).unwrap();
    let (err, _) = project_build(tmp.path());
    assert!(err.contains(PASSED), "g's gate must not see h, got:\n{err}");
    assert!(!err.contains("`Bad`"), "got:\n{err}");
    std::fs::create_dir_all(tmp.path().join("games/sonic4")).unwrap();
    std::fs::write(tmp.path().join("games/sonic4/map.toml"), "").unwrap();
    let abs = tmp.path().canonicalize().unwrap();
    let legacy = sigil(tmp.path(), &["--aeon", abs.to_str().unwrap(), "--game", "sonic4", "-o", "s4.bin"], false);
    let lerr = String::from_utf8_lossy(&legacy.stderr);
    assert!(lerr.contains("[proc.clobber-undeclared] `Bad` destroys `d2`"), "the whole-scan gate sees h's violation, got:\n{lerr}");
}

/// A game that does not declare a constant the engine requires is refused BY NAME, naming
/// the constant, the engine description's `required_consts` row and the game, on every
/// `--project` run: the build (with the gate on or opted out), `--check` and a report.
#[test]
fn a_game_missing_a_required_constant_is_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |rel, t| if rel == "games/g/gconst.emp" { "module games.g.gconst\npub const OTHER = -1\n".into() } else { t });
    let home = tmp.path().canonicalize().unwrap();
    let runs: [(&[&str], bool); 4] = [
        (&["-o", "g.bin"], false),
        (&["-o", "g.bin"], true),
        (&["--check"], false),
        (&["--report", "contracts"], false),
    ];
    for (extra, off) in runs {
        let mut args = vec!["--project", "games/g", "--shape", "release"];
        args.extend_from_slice(extra);
        let out = sigil(tmp.path(), &args, off);
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{args:?}: got:\n{err}");
        assert!(err.contains("[engine.required-const] game `g`"), "{args:?}: got:\n{err}");
        assert!(err.contains("`REQ_CONST`"), "{args:?}: names the constant, got:\n{err}");
        assert!(err.contains(&format!("{} `required_consts`", home.join("engine/engine.toml").display())), "{args:?}: names the row, got:\n{err}");
        assert!(err.contains("(games/g/game.toml)"), "{args:?}: names the game's description, got:\n{err}");
        assert!(!err.contains("contract closure gate"), "{args:?}: refused before the gate, got:\n{err}");
    }
}

/// No engine default (G0b Task 9): an ENGINE module that declares a constant the engine's
/// `required_consts` names is refused by name, even though the game declares it too (the
/// engine's value would be a default a game that forgot it builds on). Refused on every
/// `--project` run, before the gate.
#[test]
fn an_engine_default_for_a_required_constant_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |rel, t| if rel == "engine/eng.emp" { format!("{t}pub const REQ_CONST = 0\n") } else { t });
    let home = tmp.path().canonicalize().unwrap();
    for extra in [&["-o", "g.bin"][..], &["--check"][..]] {
        let mut args = vec!["--project", "games/g", "--shape", "release"];
        args.extend_from_slice(extra);
        let out = sigil(tmp.path(), &args, false);
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{args:?}: got:\n{err}");
        assert!(err.contains("[engine.required-const-default] the engine declares `REQ_CONST` in `engine.eng`"), "{args:?}: got:\n{err}");
        assert!(err.contains(&home.join("engine/eng.emp").display().to_string()), "{args:?}: names the file, got:\n{err}");
        assert!(err.contains(&format!("{} `required_consts`", home.join("engine/engine.toml").display())), "{args:?}: names the row, got:\n{err}");
        assert!(!err.contains("contract closure gate"), "{args:?}: refused before the gate, got:\n{err}");
    }
}

/// The other game is not even parsed (G0b Task 9, closes G0a A-I2): a module in `h`'s
/// sources that would break any scan of the whole checkout (it redeclares the engine's
/// module id) does not reach `g`'s build, whose gate passes; the profile-driven driver,
/// which walks the checkout, stops on it.
#[test]
fn another_games_broken_module_is_not_scanned() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |_, t| t);
    std::fs::write(tmp.path().join("games/h/dup.emp"), "module engine.eng\n").unwrap();
    let (err, _) = project_build(tmp.path());
    assert!(err.contains(PASSED), "g's build must not scan h, got:\n{err}");
    assert!(!err.contains("declared"), "got:\n{err}");
    std::fs::create_dir_all(tmp.path().join("games/sonic4")).unwrap();
    std::fs::write(tmp.path().join("games/sonic4/map.toml"), "").unwrap();
    let abs = tmp.path().canonicalize().unwrap();
    let legacy = sigil(tmp.path(), &["--aeon", abs.to_str().unwrap(), "--game", "sonic4", "-o", "s4.bin"], false);
    let lerr = String::from_utf8_lossy(&legacy.stderr);
    assert_eq!(legacy.status.code(), Some(1), "the whole-checkout scan must stop on h's module, got:\n{lerr}");
    assert!(lerr.contains("engine.eng"), "the whole-checkout scan names the redeclared id, got:\n{lerr}");
}

/// A stale row in the game's own file is GONE, refused.
#[test]
fn a_stale_row_in_the_games_file_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |rel, t| if rel == "games/g/baseline.toml" { "format = 1\nout_unverified = [[\"Callee\", \"d0\"]]\n".into() } else { t });
    let (err, out) = project_build(tmp.path());
    assert_eq!(out.status.code(), Some(1), "got:\n{err}");
    assert!(err.contains("[proc.out-unverified] moved against the frozen baseline."), "got:\n{err}");
    assert!(err.contains("\"Callee :: out(d0) (got 0, want 1)\""), "got:\n{err}");
}

/// Another game's row on an ENGINE proc does not join a `g` shape: with the row only in
/// `h`'s file (which `g` never reads), `g` sees `EngCaller` as NEW.
#[test]
fn another_games_row_on_an_engine_proc_does_not_join() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |rel, t| if rel == "engine/baseline.toml" { "format = 1\n".into() } else { t });
    let (err, out) = project_build(tmp.path());
    assert_eq!(out.status.code(), Some(1), "got:\n{err}");
    assert!(err.contains("\"EngCaller @ Callee :: d0 (got 1, want 0)\""), "got:\n{err}");
}

/// `SIGIL_CONTRACTS=0` skips the gate on `--project` too, loudly.
#[test]
fn sigil_contracts_0_skips_the_project_gate_loudly() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |rel, t| if rel == "engine/eng.emp" { format!("{t}{VIOLATION}") } else { t });
    let out = sigil(tmp.path(), &["--project", "games/g", "--shape", "release", "-o", "g.bin"], true);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("warning: contract closure gate SKIPPED (SIGIL_CONTRACTS=0)."), "got:\n{err}");
    assert!(!err.contains(FAILED) && !err.contains("contract closure gate: passed"), "got:\n{err}");
}

/// `--report contracts` on `--project` prints the census the gate reads, headed by the
/// description's names: one D1c firing, the engine's (`h`'s proc is not analysed).
#[test]
fn the_project_contract_report_prints_the_census() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), |_, t| t);
    let out = sigil(tmp.path(), &["--project", "games/g", "--shape", "release", "--report", "contracts"], false);
    let report = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "stderr:\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(report.starts_with("contract closure, g release\n"), "got:\n{report}");
    assert!(report.contains("-- [call.live-clobbered] firings (D1c, 1): --"), "got:\n{report}");
}

/// The mode pairings refuse with the profile-driven grammar's words.
#[test]
fn project_report_and_check_refuse_what_the_legacy_grammar_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let cases: [(&[&str], &str); 4] = [
        (&["--report", "contracts", "-o", "x.bin"], "--report prints instead of building; drop -o / --emit-lst"),
        (&["--check", "-o", "x.bin"], "--check decides guards without writing a ROM; drop -o / --emit-lst"),
        (&["--check", "--report", "ram"], "--check and --report are different runs; pick one"),
        (&[], "--project needs -o <out.bin> (or --report <kind> / --check)"),
    ];
    for (extra, want) in cases {
        let mut args = vec!["--project", "games/g", "--shape", "release"];
        args.extend_from_slice(extra);
        let out = sigil(tmp.path(), &args, false);
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {err}");
        assert!(err.contains(want), "{args:?}: want {want:?}, got:\n{err}");
    }
}
