//! The installed `emit_sound_blob` must emit with the checkout it was COMPILED in gone.
//!
//! The sibling of `crates/sigil-cli/tests/installed_binary_needs_no_build_tree.rs`, which
//! holds the same property for `sigil`: aeon's `build.sh` runs both binaries from a copy
//! installed out of a sigil worktree, and a run-time read resolved against the compiling
//! checkout broke every aeon build when that worktree was removed (2026-09-26). This gate
//! runs the real binary with the whole compiling checkout HIDDEN (`bwrap`, an empty tmpfs
//! mounted over it) and requires it to emit every artifact byte for byte as it does with
//! the checkout visible.
//!
//! Controls, each asserted: the hiding is real (a committed file is absent inside the
//! sandbox and present outside it, with the arguments the run uses); the run measured
//! something (the visible run wrote at least one non-empty artifact, and the hidden run
//! wrote exactly the same file set). Unmeasurable is loud: no `bwrap`, or a reference tree
//! inside the checkout, fails naming why; no reference tree follows the house pattern
//! (skip, or a failure under `SIGIL_STRICT_GATE=1`).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon \
//!   cargo test --release -p sigil-harness --test installed_binary_needs_no_build_tree
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A file every checkout of this repository carries, used as the hiding control.
const CONTROL_FILE: &str = "crates/sigil-harness/golden/provenance.toml";

fn fresh_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("installed_binary_needs_no_build_tree_{case}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.canonicalize().expect("canonical scratch dir")
}

/// A `bwrap` command that shows the whole filesystem except `hidden`, which is an empty
/// tmpfs, with each of `keep` bound back at its own path when it lies inside `hidden`.
fn sandboxed(hidden: &Path, keep: &[&Path]) -> Command {
    let mut cmd = Command::new("bwrap");
    cmd.args(["--dev-bind", "/", "/", "--tmpfs"]).arg(hidden);
    for k in keep {
        if k.starts_with(hidden) {
            cmd.arg("--bind").arg(k).arg(k);
        }
    }
    cmd.args(["--die-with-parent", "--"]);
    cmd
}

/// Run the emitter into `out` and return every file it wrote, by name.
fn emit(mut cmd: Command, emitter: &Path, aeon: &Path, out: &Path, label: &str) -> BTreeMap<String, Vec<u8>> {
    cmd.arg(emitter).arg("--aeon").arg(aeon).arg("--out-dir").arg(out);
    cmd.env("AEON_DIR", aeon);
    let o = cmd.output().unwrap_or_else(|e| panic!("spawn the {label} emit: {e}"));
    assert!(
        o.status.success(),
        "{label}: `emit_sound_blob` failed ({}):\n{}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(out).unwrap_or_else(|e| panic!("{label}: list {}: {e}", out.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_file() {
            let name = path.file_name().expect("a file name").to_string_lossy().into_owned();
            files.insert(name, std::fs::read(&path).expect("read an emitted artifact"));
        }
    }
    files
}

#[test]
fn installed_binary_needs_no_build_tree() {
    let profile = sigil_harness::native::sonic4_profile(false);
    let Some(aeon) = sigil_harness::test_support::reference_tree_for_profile(&profile) else { return };
    let aeon = aeon.canonicalize().expect("canonical aeon");
    let checkout = sigil_harness::source_digest::sigil_source_root();
    let emitter =
        PathBuf::from(env!("CARGO_BIN_EXE_emit_sound_blob")).canonicalize().expect("canonical emitter");
    let out = fresh_dir("emit");

    let probe = Command::new("bwrap").arg("--version").output();
    assert!(
        matches!(&probe, Ok(o) if o.status.success()),
        "UNMEASURABLE: `bwrap` is not runnable here ({probe:?}), and it is what hides the compiling \
         checkout from the binary. This is not a pass."
    );
    assert!(
        !aeon.starts_with(&checkout),
        "UNMEASURABLE: the reference tree {} lies inside the compiling checkout {}, so hiding the \
         checkout would hide the emitter's own input. Point AEON_DIR outside it.",
        aeon.display(),
        checkout.display()
    );

    let control = checkout.join(CONTROL_FILE);
    assert!(control.is_file(), "the control file {} is not in this checkout", control.display());
    let seen = sandboxed(&checkout, &[&emitter, &out])
        .arg("test")
        .arg("-e")
        .arg(&control)
        .status()
        .expect("spawn bwrap for the hiding control");
    assert!(
        !seen.success(),
        "the sandbox still shows {}, so it does not hide the compiling checkout and the emit \
         below would prove nothing",
        control.display()
    );

    let open_dir = out.join("visible");
    let hidden_dir = out.join("hidden");
    std::fs::create_dir_all(&open_dir).expect("visible out dir");
    std::fs::create_dir_all(&hidden_dir).expect("hidden out dir");
    let visible = emit(Command::new("env"), &emitter, &aeon, &open_dir, "visible");
    let hidden = emit(sandboxed(&checkout, &[&emitter, &out]), &emitter, &aeon, &hidden_dir, "hidden");

    assert!(
        visible.values().any(|b| !b.is_empty()),
        "the visible emit wrote no non-empty artifact, so the comparison below measures nothing: {:?}",
        visible.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        visible.keys().collect::<Vec<_>>(),
        hidden.keys().collect::<Vec<_>>(),
        "the emit wrote a different file set with the compiling checkout hidden"
    );
    let differing: Vec<&String> = visible.keys().filter(|k| visible[*k] != hidden[*k]).collect();
    assert!(
        differing.is_empty(),
        "with the compiling checkout hidden the emit wrote different bytes for {differing:?}, so the \
         binary still reads something from it"
    );
    eprintln!(
        "emit_sound_blob: {} artifact(s) byte-identical with {} hidden",
        visible.len(),
        checkout.display()
    );
    let _ = std::fs::remove_dir_all(&out);
}
