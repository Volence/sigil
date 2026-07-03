//! Integration smoke test for the `sigil build` / `sigil diff` subcommands.
//!
//! `sigil diff --aeon <aeon>` assembles the M0 integration harness's Region
//! A + Region B reference source and compares it byte-for-byte against the
//! committed golden blobs (see `sigil-harness`). This test reads the aeon
//! source tree, so it's `#[ignore]`d by default — run with `--ignored`.

use std::process::Command;

#[test]
#[ignore = "reads the aeon source tree; run with --ignored"]
fn sigil_diff_reports_byte_identity() {
    let aeon = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../aeon");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["diff", "--aeon", aeon])
        .output()
        .expect("run sigil diff");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
}
