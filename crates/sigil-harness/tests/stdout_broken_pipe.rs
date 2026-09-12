//! `refreeze`, whose stdout reader has gone, stops writing and ends the run with
//! the status it would have had with a live reader, printing nothing about the
//! pipe: the rule `sigil_harness::stdout` states, held for this crate's binaries.
//! `crates/sigil-cli/tests/stdout_broken_pipe.rs` holds `sigil` and `emp_census`
//! to it, and says why each assertion's shape is the one that tells the rule apart
//! from every other way of meeting a closed stdout.
//!
//! `refreeze` is the one binary of this crate with a multi-line stdout reachable
//! without a reference tree. `repin`, `emit_sound_blob`, `cycle_fraction` and
//! `derive_offcanon` each need an aeon tree before their first stdout write, so
//! the import that puts them under the rule is held by
//! `tests/stdout_writer_population.rs` rather than by a run.
//!
//! # The fixture
//!
//! A harness root holding the two markers, `golden/provenance.toml` (a copy of the
//! committed ledger, read and never written) and an empty `repin.toml`, outside
//! any git repository and with `AEON_DIR` unset. Both of `refreeze`'s revision
//! oracles then answer `COULD NOT MEASURE` without spawning a network call, so the
//! run is fast and its output is the same every time. It ends with status 2, which
//! is the point: a closed stdout must leave a nonzero status exactly as it found it.

use std::path::Path;
use std::process::{Command, Output, Stdio};

const REFREEZE: &str = env!("CARGO_BIN_EXE_refreeze");

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `refreeze <args>` against the fixture root, with no route to a repository.
fn refreeze(root: &Path, args: &[&str]) -> Command {
    let mut c = Command::new(REFREEZE);
    c.args(args)
        .current_dir(root)
        .env("SIGIL_HARNESS_ROOT", root)
        // No repository is found at or above the root, whatever directory the
        // temporary one sits in, so no oracle reaches `git ls-remote`.
        .env("GIT_CEILING_DIRECTORIES", root.parent().expect("the root has a parent"))
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("AEON_DIR");
    c
}

/// With stdout the write end of a pipe whose read end is already closed.
fn closed(mut cmd: Command) -> Output {
    let (reader, writer) = std::io::pipe().expect("create a pipe");
    drop(reader);
    cmd.stdout(Stdio::from(writer))
        .stderr(Stdio::piped())
        .output()
        .expect("spawn refreeze")
}

#[test]
fn refreeze_into_a_closed_pipe_ends_as_it_would_have() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("root");
    std::fs::create_dir_all(root.join("golden")).expect("mkdir golden");
    let ledger = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("golden/provenance.toml"))
        .expect("read the committed provenance.toml");
    std::fs::write(root.join("golden/provenance.toml"), ledger).expect("write the ledger copy");
    std::fs::write(root.join("repin.toml"), "").expect("write repin.toml");

    for mode in ["--reachability", "--check"] {
        let live = refreeze(&root, &[mode]).output().expect("spawn refreeze");
        let shut = closed(refreeze(&root, &[mode]));
        let report = text(&live.stdout);
        assert!(
            report.starts_with(&format!("refreeze {mode}: ")) && report.lines().count() > 1,
            "{mode}: a live reader did not get the multi-line report, so a closed stdout had \
             nothing to refuse. stdout:\n{report}\nstderr:\n{}",
            text(&live.stderr)
        );
        assert_eq!(
            live.status.code(),
            Some(2),
            "{mode}: the fixture must end at status 2 (nothing measurable), or a status the \
             closed run could fall back to is being compared. stderr:\n{}",
            text(&live.stderr)
        );
        assert_eq!(
            shut.status.code(),
            live.status.code(),
            "{mode}: closing stdout changed how the run ended ({:?}, against a live reader's \
             {:?}). stderr of the closed run:\n{}",
            shut.status,
            live.status,
            text(&shut.stderr)
        );
        assert_eq!(text(&shut.stderr), text(&live.stderr), "{mode}: closing stdout changed stderr");
    }
}
