//! A private copy of the reference tree for a whole-build gate test (A10), in the
//! `build_check.rs` pattern: copied per test, deleted on the way out.
//!
//! The copy lives at the FIXED path `CARGO_TARGET_TMPDIR/<name>`, so a copy left by a killed
//! run is wiped by the next setup rather than accumulating (the tree is ~139 MB). The guard
//! holds the binary's lock for the whole test, so no two tests share or delete one copy, and
//! its `Drop` removes the copy even when the test panics.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};

// Every test in one binary uses the same fixed path; serialize them.
static LOCK: Mutex<()> = Mutex::new(());

pub struct GateTree {
    root: PathBuf,
    aeon: PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl GateTree {
    /// Copy the demo-shape reference tree to `CARGO_TARGET_TMPDIR/<name>/tree`, first wiping
    /// whatever an earlier (killed) run left there. `None` when no reference tree is present.
    pub fn new(name: &str) -> Option<Self> {
        let reference =
            sigil_harness::test_support::reference_tree_for_shape(&sigil_harness::shapes::Shape::new("demo", "release"))?;
        let lock = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let aeon = root.join("tree");
        sigil_harness::test_support::copy_tree(&reference, &aeon);
        Some(Self { root, aeon, _lock: lock })
    }

    /// The text of `rel` in the copy.
    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.aeon.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
    }

    /// Overwrite `rel` in the copy.
    pub fn write(&self, rel: &str, text: &str) {
        std::fs::write(self.aeon.join(rel), text).unwrap_or_else(|e| panic!("write {rel}: {e}"));
    }

    /// Run `sigil build --project <copy>/games/demo --shape release <args>`. A lone `-o` in
    /// `args` is followed by the ROM path, removed first. `AEON_DIR`, `SIGIL_CONTRACTS` and
    /// `SIGIL_WARNINGS` are removed from the child, then `env` is applied. Returns
    /// (output, stderr, rom path).
    pub fn build(&self, args: &[&str], env: &[(&str, &str)]) -> (Output, String, PathBuf) {
        let rom = self.root.join("probe.bin");
        let _ = std::fs::remove_file(&rom);
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
        cmd.current_dir(&self.root)
            .args(["build", "--project"])
            .arg(self.aeon.join("games").join("demo"))
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
        (out, err, rom)
    }
}

impl Drop for GateTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
