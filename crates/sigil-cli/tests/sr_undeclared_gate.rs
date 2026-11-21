//! A10 W7: an undeclared write to the status register stops the GATED `sigil build`.
//!
//! `[proc.sr-undeclared]` stays a warning at both emission sites (`lower/proc.rs` and
//! `eval/asm.rs`). The gated arm of `sigil build --project ... -o ...` (the contract closure
//! gate ran, so `SIGIL_CONTRACTS` is not `0`) refuses when its own warning list holds that id,
//! so `SIGIL_WARNINGS=off`, which only changes what is printed, cannot hide it.
//!
//! The card's limit, pinned here: "an error only where the gate runs; the single-file path is
//! a separate question (review B, Q15)". `--check`, `SIGIL_CONTRACTS=0` and single-file
//! `sigil emp` keep the warning.
//!
//! The gate fixture `write_project_fixture` cannot drive this: its build stops before lowering,
//! so no warning list exists. These tests plant a proc in `games/demo/demo_state.emp` of ONE
//! copy of the reference tree per test binary, as `build_check.rs` does.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon \
//!   cargo test --release -p sigil-cli --test sr_undeclared_gate
//! ```

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};

const PLANT: &str = "\npub proc Zz_A10_SrProbe () clobbers(d0) {\n        moveq   #0, d0\n        move.w  #$2700, sr\n        rts\n}\n";
const TWIN: &str = "\npub proc Zz_A10_SrProbe () clobbers(d0/sr) {\n        moveq   #0, d0\n        move.w  #$2700, sr\n        rts\n}\n";
const HEADER: &str = "error: [proc.sr-undeclared], 1 firing(s); an undeclared write to the status register stops a gated `sigil build`:";

const STATE: &str = "games/demo/demo_state.emp";

// Every run rewrites the same copy; serialize so one run's tree and output are its own.
static LOCK: Mutex<()> = Mutex::new(());
// The copy of the reference tree and the pristine `demo_state.emp`, made once per binary.
static TREE: OnceLock<Option<(PathBuf, String)>> = OnceLock::new();

fn tree() -> Option<&'static (PathBuf, String)> {
    TREE.get_or_init(|| {
        let reference =
            sigil_harness::test_support::reference_tree_for_shape(&sigil_harness::shapes::Shape::new("demo", "release"))?;
        let pristine = std::fs::read_to_string(reference.join(STATE)).expect("read demo_state.emp");
        let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("sr_undeclared_gate_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&copy);
        std::fs::create_dir_all(&copy).unwrap();
        let aeon = copy.join("tree");
        sigil_harness::test_support::copy_tree(&reference, &aeon);
        Some((aeon, pristine))
    })
    .as_ref()
}

/// Run `sigil build --project <copy>/games/demo --shape release <args>` with `plant` appended
/// to the pristine `demo_state.emp`. A lone `-o` in `args` is followed by the ROM path, which is
/// removed first. Returns (output, stderr, rom path); `None` when no reference tree is present.
fn build(plant: &str, args: &[&str], env: &[(&str, &str)]) -> Option<(Output, String, PathBuf)> {
    let (aeon, pristine) = tree()?;
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::fs::write(aeon.join(STATE), format!("{pristine}{plant}")).unwrap();
    let rom = aeon.parent().unwrap().join("sr_probe.bin");
    let _ = std::fs::remove_file(&rom);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.current_dir(aeon.parent().unwrap())
        .args(["build", "--project"])
        .arg(aeon.join("games").join("demo"))
        .args(["--shape", "release"]);
    for a in args {
        cmd.arg(a);
        if *a == "-o" {
            cmd.arg(&rom);
        }
    }
    cmd.env_remove("AEON_DIR");
    cmd.env_remove("SIGIL_CONTRACTS");
    cmd.env_remove("SIGIL_WARNINGS");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("run sigil build");
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    Some((out, err, rom))
}

#[test]
fn an_undeclared_sr_write_stops_the_gated_build() {
    let Some((out, err, rom)) = build(PLANT, &["-o"], &[]) else { return };
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains(HEADER), "{err}");
    let line = err
        .lines()
        .find(|l| l.contains("[proc.sr-undeclared] `Zz_A10_SrProbe` writes `sr`"))
        .unwrap_or_else(|| panic!("{err}"));
    assert!(line.contains("games/demo/demo_state.emp:"), "located: {line}");
    assert!(err.contains("contract closure gate: passed"), "the closure gate itself ran and passed first:\n{err}");
    assert!(!rom.exists(), "a refused build writes no ROM");
}

/// Review Focus 5: silencing the warn tier does not silence the refusal.
#[test]
fn warnings_off_does_not_hide_the_refusal() {
    let Some((out, err, _)) = build(PLANT, &["-o"], &[("SIGIL_WARNINGS", "off")]) else { return };
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains(HEADER), "{err}");
}

/// The control: the same write, declared, builds.
#[test]
fn a_declared_sr_write_builds() {
    let Some((out, err, rom)) = build(TWIN, &["-o"], &[]) else { return };
    assert!(out.status.success(), "{err}");
    assert!(!err.contains("proc.sr-undeclared"), "{err}");
    assert!(rom.exists());
}

/// The card's limit, pinned: where the gate does not run it stays a warning.
#[test]
fn outside_the_gate_it_stays_a_warning() {
    let Some((out, err, rom)) = build(PLANT, &["-o"], &[("SIGIL_CONTRACTS", "0")]) else { return };
    assert!(out.status.success() && rom.exists(), "{err}");
    assert!(err.contains("proc.sr-undeclared 1") && !err.contains(HEADER), "{err}");
    let Some((out, err, _)) = build(PLANT, &["--check"], &[]) else { return };
    assert!(out.status.success(), "{err}");
    assert!(err.contains("proc.sr-undeclared 1") && !err.contains(HEADER), "{err}");
}

/// The single-file path (review B, Q15) is not tightened by A10. No reference tree needed.
#[test]
fn the_single_file_path_stays_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("w7.emp");
    std::fs::write(&src, "module w7\nproc P () clobbers(d0) {\n    moveq   #0, d0\n    move.w  #$2700, sr\n    rts\n}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg("emp")
        .arg(&src)
        .arg("-o")
        .arg(dir.path().join("w7.bin"))
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{err}");
    assert!(err.contains("[proc.sr-undeclared] `P` writes `sr`"), "{err}");
    assert_eq!(std::fs::read(dir.path().join("w7.bin")).unwrap(), [0x70, 0x00, 0x46, 0xFC, 0x27, 0x00, 0x4E, 0x75]);
}
