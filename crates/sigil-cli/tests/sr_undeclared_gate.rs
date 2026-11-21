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
//! so no warning list exists. Each test plants a proc in `games/demo/demo_state.emp` of its
//! own copy of the reference tree, deleted on the way out, as `build_check.rs` does
//! (`support/gate_tree.rs`).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon \
//!   cargo test --release -p sigil-cli --test sr_undeclared_gate
//! ```

#[path = "support/gate_tree.rs"]
mod gate_tree;

use std::process::Command;

const PLANT: &str = "\npub proc Zz_A10_SrProbe () clobbers(d0) {\n        moveq   #0, d0\n        move.w  #$2700, sr\n        rts\n}\n";
const TWIN: &str = "\npub proc Zz_A10_SrProbe () clobbers(d0/sr) {\n        moveq   #0, d0\n        move.w  #$2700, sr\n        rts\n}\n";
const HEADER: &str = "error: [proc.sr-undeclared], 1 firing(s); an undeclared write to the status register stops a gated `sigil build`:";

const STATE: &str = "games/demo/demo_state.emp";

/// A fresh copy of the reference tree with `plant` appended to `demo_state.emp`, deleted when
/// the returned guard drops (see `support/gate_tree.rs`). `None` when no reference tree is present.
fn planted(plant: &str) -> Option<gate_tree::GateTree> {
    let tree = gate_tree::GateTree::new("sr_undeclared_gate")?;
    let pristine = tree.read(STATE);
    tree.write(STATE, &format!("{pristine}{plant}"));
    Some(tree)
}

#[test]
fn an_undeclared_sr_write_stops_the_gated_build() {
    let Some(tree) = planted(PLANT) else { return };
    let (out, err, rom) = tree.build(&["-o"], &[]);
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
    let Some(tree) = planted(PLANT) else { return };
    let (out, err, rom) = tree.build(&["-o"], &[("SIGIL_WARNINGS", "off")]);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains(HEADER), "{err}");
    assert!(!rom.exists(), "a refused build writes no ROM");
}

/// The control: the same write, declared, builds.
#[test]
fn a_declared_sr_write_builds() {
    let Some(tree) = planted(TWIN) else { return };
    let (out, err, rom) = tree.build(&["-o"], &[]);
    assert!(out.status.success(), "{err}");
    assert!(!err.contains("proc.sr-undeclared"), "{err}");
    assert!(rom.exists());
}

/// The card's limit, pinned: where the gate does not run it stays a warning.
#[test]
fn outside_the_gate_it_stays_a_warning() {
    let Some(tree) = planted(PLANT) else { return };
    let (out, err, rom) = tree.build(&["-o"], &[("SIGIL_CONTRACTS", "0")]);
    assert!(out.status.success() && rom.exists(), "{err}");
    assert!(err.contains("proc.sr-undeclared 1") && !err.contains(HEADER), "{err}");
    let (out, err, _) = tree.build(&["--check"], &[]);
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
