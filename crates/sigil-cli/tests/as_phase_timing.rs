//! `SIGIL_PHASE_TIMING`: the AS route's per-phase timing lines on stderr.
//!
//! What each test catches:
//!
//! - `lines_appear_with_the_variable_set`: a gate that never opens, a phase that
//!   lost its line, a line missing its wall time or memory fields, or a pass line
//!   whose `include` accounting does not count the one `include` each pass reads.
//! - `nothing_appears_without_it`: a gate that is always open, so every build
//!   prints the lines.
//! - `the_image_does_not_depend_on_it`: timing that changes what is built.

use std::path::Path;
use std::process::{Command, Output};

const VAR: &str = "SIGIL_PHASE_TIMING";

/// A root with one `include`, one `BINCLUDE` and a forward reference, so every
/// pass reads a file of each kind and the loop runs more than one pass.
fn tree(dir: &Path) {
    std::fs::write(
        dir.join("root.asm"),
        "\tcpu 68000\n\tdc.w\tLater\n\tinclude \"part.asm\"\n\tbinclude \"blob.bin\"\nLater:\n\tdc.b\t1\n",
    )
    .expect("write root.asm");
    std::fs::write(dir.join("part.asm"), "\tdc.b\t2,3\n").expect("write part.asm");
    std::fs::write(dir.join("blob.bin"), [0xAAu8, 0xBB]).expect("write blob.bin");
}

fn run(dir: &Path, timing: bool) -> (Output, Vec<u8>) {
    let out_path = dir.join(if timing { "timed.bin" } else { "plain.bin" });
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.arg(dir.join("root.asm")).arg("-o").arg(&out_path).arg("-p=FF");
    if timing {
        cmd.env(VAR, "1");
    } else {
        cmd.env_remove(VAR);
    }
    let out = cmd.output().expect("spawn sigil");
    assert!(
        out.status.success(),
        "the build must succeed.\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let image = std::fs::read(&out_path).expect("the image was written");
    (out, image)
}

/// Every `SIGIL-PHASE` line as (name, fields).
fn phase_lines(stderr: &str) -> Vec<(String, Vec<(String, String)>)> {
    stderr
        .lines()
        .filter_map(|l| l.strip_prefix("SIGIL-PHASE\t"))
        .map(|rest| {
            let mut cols = rest.split('\t');
            let name = cols.next().unwrap_or_default().to_string();
            let fields = cols
                .filter_map(|c| c.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            (name, fields)
        })
        .collect()
}

fn field<'a>(fields: &'a [(String, String)], key: &str) -> Option<&'a str> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

#[test]
fn lines_appear_with_the_variable_set() {
    let dir = tempfile::tempdir().expect("tempdir");
    tree(dir.path());
    let (out, _) = run(dir.path(), true);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let lines = phase_lines(&stderr);
    let names: Vec<&str> = lines.iter().map(|(n, _)| n.as_str()).collect();
    for want in [
        "cli.args",
        "frontend.read_root",
        "  pass.seed",
        "  pass.exec",
        "  pass.finish",
        "pass0",
        "pass1",
        "frontend",
        "layout",
        "link",
        "flatten",
        "header",
        "emit",
        "teardown",
        "total",
    ] {
        assert!(names.contains(&want), "no `{want}` phase line.\nstderr:\n{stderr}");
    }
    for (name, fields) in &lines {
        for key in ["wall_ms", "hwm_kb", "rss_kb"] {
            let v = field(fields, key).unwrap_or_else(|| panic!("`{name}` has no {key}.\nstderr:\n{stderr}"));
            assert!(v.parse::<f64>().is_ok(), "`{name}` {key}={v} is not a number");
        }
    }
    let passes = names.iter().filter(|n| n.starts_with("pass") && !n.contains('.')).count();
    let execs: Vec<_> = lines.iter().filter(|(n, _)| n == "  pass.exec").collect();
    assert_eq!(execs.len(), passes, "one exec step per pass.\nstderr:\n{stderr}");
    for (_, fields) in execs {
        assert_eq!(field(fields, "include_read_calls"), Some("1"), "stderr:\n{stderr}");
        assert_eq!(field(fields, "binclude_read_calls"), Some("1"), "stderr:\n{stderr}");
        assert_eq!(field(fields, "split_lines_calls"), Some("2"), "root and include.\nstderr:\n{stderr}");
    }
}

#[test]
fn nothing_appears_without_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    tree(dir.path());
    let (out, _) = run(dir.path(), false);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("SIGIL-PHASE"), "phase lines without {VAR}.\nstderr:\n{stderr}");
}

#[test]
fn the_image_does_not_depend_on_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    tree(dir.path());
    let (_, timed) = run(dir.path(), true);
    let (_, plain) = run(dir.path(), false);
    assert_eq!(timed, plain);
    assert_eq!(&plain[..2], &[0x00, 0x06], "`Later` resolved forward: {plain:02X?}");
}
