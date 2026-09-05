//! The plain `sigil <file.asm>` path must resolve `include` relative to the SOURCE
//! FILE, not to the directory the command happened to be run from.
//!
//! ## Why this has its own gate
//!
//! `sigil_frontend_as` already carries both behaviours and documents the difference:
//! `assemble(&str)` leaves `Options::include_root` at `None`, while `assemble_root(&Path)`
//! is *"Assemble a root source file, resolving `include` paths relative to its parent
//! directory"*. Every in-tree caller that assembles a real multi-file program reaches the
//! second. The bare CLI reached the first, so an `include` was looked up under the
//! invoking shell's cwd.
//!
//! That is invisible in this workspace, because the harness never invokes the bare CLI on
//! a foreign tree — and it is the FIRST thing an outside user does. Measured on the
//! community Sonic 2 disassembly: run from its own directory it reports 237 diagnostics;
//! run from one directory up it reports **59,122**, beginning with five bogus
//! `cannot include` lines and continuing with the whole file misparsed as a cascade from
//! the constants and macros that never loaded. Nothing in that output says the paths were
//! resolved against the wrong directory, so the failure reads as "this assembler cannot
//! handle my project".
//!
//! The assertion below is deliberately about the CLI PROCESS rather than about
//! `Options::include_root`: the field was always reachable, and a unit test on the library
//! would have passed throughout while the shipped command was broken.

use std::process::Command;

/// A root that includes a sibling, assembled from an unrelated working directory.
#[test]
fn include_resolves_against_the_source_file_not_the_current_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path();

    std::fs::write(
        src.join("part.asm"),
        "\tdc.w $1234\n",
    )
    .expect("write part.asm");
    std::fs::write(
        src.join("root.asm"),
        "\tcpu 68000\n\tinclude \"part.asm\"\n",
    )
    .expect("write root.asm");

    // The control that makes the assertion mean something: run from a directory that is
    // NOT the source's own, and that demonstrably does not contain the included file.
    let elsewhere = tempfile::tempdir().expect("second tempdir");
    assert!(
        !elsewhere.path().join("part.asm").exists(),
        "the cwd must not contain part.asm, or this test passes for the wrong reason"
    );

    let out_bin = src.join("root.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .current_dir(elsewhere.path())
        .args([
            src.join("root.asm").to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .output()
        .expect("spawn sigil");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("cannot include"),
        "`include` was resolved against the current directory rather than the source \
         file's own directory. stderr:\n{stderr}"
    );
    assert!(
        out.status.success(),
        "assembling a root with a sibling include from an unrelated cwd must succeed.\n\
         status: {:?}\nstderr:\n{stderr}",
        out.status.code()
    );

    // The include really was consumed, rather than the run succeeding by skipping it.
    let bytes = std::fs::read(&out_bin).expect("output written");
    assert!(
        bytes.windows(2).any(|w| w == [0x12, 0x34]),
        "the included `dc.w $1234` is absent from the output, so the include did not \
         contribute, a pass here would be vacuous. bytes: {bytes:02x?}"
    );
}
