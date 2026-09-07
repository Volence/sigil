//! `sigil <root.asm>` writes the AS `message` directive's lines to STDOUT,
//! unprefixed, one per line, and nothing else about them anywhere.
//!
//! That is asl's contract for the directive (reference build md5
//! `61e672562465725a8c102288a7da9098`, probe `p1b`: `message "int \{42}"`
//! prints the bare line `int 2A` to stdout, exit 0, no diagnostic), and it is
//! the contract a build script depends on: s1disasm reads its Z80 driver size
//! and s2disasm its ROM size off this line. A `file(line): note:` rendering
//! on stderr would satisfy a human and break the reader.
//!
//! The pre-fix binary printed nothing for any of these programs.

use std::process::{Command, Output};

/// Assemble `body` (after a `cpu 68000` line) from a fresh directory.
fn run(body: &str) -> Output {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("\tcpu 68000\n\tpadding off\n\torg 0\n{body}")).expect("write");
    Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg(&path)
        .output()
        .expect("spawn sigil")
}

#[test]
fn a_message_is_a_bare_stdout_line_and_not_a_diagnostic() {
    let out = run("\tmessage \"int \\{42}\"\n\tdc.b 1\n\tend\n");
    assert_eq!(out.status.code(), Some(0), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "int 2A\n");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

/// The converged value, once: asl prints `fwd 0` then `fwd 3` for this
/// program, one line per pass; sigil prints the final line only.
#[test]
fn a_forward_referenced_message_prints_once_with_its_final_value() {
    let out = run("\tmessage \"fwd \\{Later}\"\n\tdc.b 1\n\tdc.w Later-*\nLater:\n\tend\n");
    assert_eq!(out.status.code(), Some(0), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "fwd 3\n");
}

/// s1disasm's shape: the line prints and the run fails afterwards. The
/// message is on stdout, the failure on stderr, exit 1.
#[test]
fn a_failing_run_prints_its_message_before_its_diagnostics() {
    let out = run("\tmessage \"size \\{Later}h bytes\"\n\tdc.b 1\n\tdc.w Later-*\nLater:\n\tdc.b nothing_defines_this\n\tend\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "size 3h bytes\n");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("nothing_defines_this"), "stderr: {err}");
    assert!(!err.contains("size 3h bytes"), "the message is not a diagnostic: {err}");
}

/// The corpus line that was uninterpolated: a parenthesised float division
/// renders as asl renders it (probe `p5b`, pass-2 line).
#[test]
fn a_float_division_in_a_message_interpolates() {
    let out = run(
        "StartOfRom:\n\tdc.b 1\n\tdc.w EndOfRom-*\npaddingSoFar equ 3\n\
         \tmessage \"ROM size is $\\{EndOfRom-StartOfRom} bytes (\\{(EndOfRom-StartOfRom)/1024.0} KiB). About $\\{paddingSoFar} bytes are padding. \"\n\
         \tdc.b 5,6\nEndOfRom:\n\tend\n",
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "ROM size is $5 bytes (0.0048828125 KiB). About $3 bytes are padding. \n"
    );
}
