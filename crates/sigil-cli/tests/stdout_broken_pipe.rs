//! A `sigil` binary whose stdout reader has gone stops writing and ends the run
//! with the status it would have had with a live reader, printing nothing about
//! the pipe. The rule, and why it is not exit 0 or SIGPIPE's 141, is in
//! `sigil_harness::stdout`; this file holds `sigil` and `emp_census` to it.
//!
//! # A reader that has gone, made certain
//!
//! A pipe is created and its read end closed BEFORE the child is spawned with the
//! write end as its stdout, so the child's first stdout write fails with EPIPE
//! every time. `sigil --version | head -1` reaches that state only when `head`
//! exits before the banner is written, which is a race: a loop over it measures
//! the defect and cannot gate it. The mid-stream case is made certain the same
//! way, by writing more than the largest pipe the kernel allows.
//!
//! # What each assertion tells apart
//!
//! Exit 0 on its own says nothing, because a binary that printed nothing exits 0
//! too. So each closed-stdout run is paired with the same invocation against a
//! live reader, and what is asserted is the relationship: the live reader got
//! output, and closing stdout changed neither the exit status nor a byte of
//! stderr. Each other way of meeting a closed stdout fails a named test here:
//!
//! - std's `println!`: status 101 and a panic on stderr, in every closed-stdout
//!   test.
//! - SIGPIPE's default disposition: the run ends on a signal with no exit code,
//!   in every closed-stdout test.
//! - exit 0 at the broken write: a failing run's status becomes 0,
//!   `a_failing_run_keeps_its_status_when_stdout_is_closed`.
//! - dropping every write: the live reader gets nothing,
//!   `a_live_reader_gets_every_byte`.
//! - ignoring every stdout error: a full device stops failing the run,
//!   `a_full_device_on_stdout_still_fails_the_run`.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const SIGIL: &str = env!("CARGO_BIN_EXE_sigil");
const EMP_CENSUS: &str = env!("CARGO_BIN_EXE_emp_census");

/// The lines every AS fixture starts with.
const ASM_HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";

/// One invocation, rebuilt for each run of it.
struct Invocation {
    what: String,
    program: &'static str,
    args: Vec<String>,
}

impl Invocation {
    fn new(what: &str, program: &'static str, args: &[&str]) -> Self {
        Invocation {
            what: what.to_string(),
            program,
            args: args.iter().map(|a| a.to_string()).collect(),
        }
    }

    fn command(&self) -> Command {
        let mut c = Command::new(self.program);
        c.args(&self.args);
        c
    }

    /// Against a live reader that reads to the end.
    fn live(&self) -> Output {
        self.command()
            .output()
            .unwrap_or_else(|e| panic!("{}: spawn: {e}", self.what))
    }

    /// With stdout the write end of a pipe whose read end is already closed.
    fn closed(&self) -> Output {
        let (reader, writer) = std::io::pipe().expect("create a pipe");
        drop(reader);
        self.command()
            .stdout(Stdio::from(writer))
            .stderr(Stdio::piped())
            .output()
            .unwrap_or_else(|e| panic!("{}: spawn: {e}", self.what))
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The rule, for one invocation: a live reader gets output, and a closed stdout
/// changes neither the exit status nor stderr. Returns both runs.
fn holds_to_the_rule(inv: &Invocation) -> (Output, Output) {
    let live = inv.live();
    let closed = inv.closed();
    assert!(
        !live.stdout.is_empty(),
        "{}: a live reader got no stdout, so a closed one had nothing to refuse and this run \
         proves nothing",
        inv.what
    );
    assert_eq!(
        closed.status.code(),
        live.status.code(),
        "{}: closing stdout changed how the run ended ({:?}, against a live reader's {:?}). \
         stderr of the closed run:\n{}",
        inv.what,
        closed.status,
        live.status,
        text(&closed.stderr)
    );
    assert_eq!(
        text(&closed.stderr),
        text(&live.stderr),
        "{}: closing stdout changed stderr",
        inv.what
    );
    (live, closed)
}

/// Write `body` to `dir/name` and return the path as an argument.
fn fixture(dir: &Path, name: &str, body: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write fixture");
    path.to_string_lossy().into_owned()
}

/// An AS source whose image is `n` bytes of `$AA`, so `--hex` prints `3n - 1`
/// bytes of hex on one line before the `built:` line.
fn big_image_asm(dir: &Path, n: usize) -> String {
    fixture(dir, "big.asm", &format!("{ASM_HEAD}\tdc.b [{n}]$AA\n\tend\n"))
}

/// The most a pipe can hold on this machine: `fs.pipe-max-size`, the ceiling on
/// what any pipe may be grown to.
fn pipe_max_size() -> usize {
    std::fs::read_to_string("/proc/sys/fs/pipe-max-size")
        .expect("read /proc/sys/fs/pipe-max-size: this suite runs on Linux")
        .trim()
        .parse()
        .expect("fs.pipe-max-size is a number")
}

/// Two `.emp` modules with comptime tests: every test in the first passes, one in
/// the second fails.
const PASSING_TESTS: &str = "module m\n\
     comptime fn sq(x: int) -> int {\n    return x * x\n}\n\
     comptime test \"squares\" {\n    ensure(sq(3) == 9, \"3^2\")\n}\n\
     comptime test \"doubles\" {\n    ensure(sq(2) * 2 == 8, \"2^3\")\n}\n";
const FAILING_TESTS: &str = "module m\n\
     comptime test \"holds\" {\n    ensure(1 == 1, \"one\")\n}\n\
     comptime test \"broken\" {\n    ensure(1 == 2, \"math failed\")\n}\n";

/// The finding: `sigil --version` into a closed pipe exits 0 and says nothing,
/// in either spelling.
#[test]
fn version_into_a_closed_pipe_exits_zero_and_says_nothing() {
    for flag in ["--version", "-V"] {
        let inv = Invocation::new(flag, SIGIL, &[flag]);
        let (live, closed) = holds_to_the_rule(&inv);
        assert_eq!(closed.status.code(), Some(0), "{flag}: stderr:\n{}", text(&closed.stderr));
        assert!(closed.stderr.is_empty(), "{flag}: stderr:\n{}", text(&closed.stderr));
        assert!(
            text(&live.stdout).lines().count() > 1,
            "{flag}: the banner a closed pipe refused is one line, so the multi-line case \
             is untested:\n{}",
            text(&live.stdout)
        );
    }
}

/// Every other multi-line stdout of a run that succeeds: help, a test report, and
/// a hex image.
#[test]
fn a_succeeding_multi_line_run_into_a_closed_pipe_ends_as_it_would_have() {
    let dir = tempfile::tempdir().expect("tempdir");
    let tests = fixture(dir.path(), "passing.emp", PASSING_TESTS);
    let big = big_image_asm(dir.path(), 4096);
    for inv in [
        Invocation::new("--help", SIGIL, &["--help"]),
        Invocation::new("help build", SIGIL, &["help", "build"]),
        Invocation::new("build --help", SIGIL, &["build", "--help"]),
        Invocation::new("test <passing.emp>", SIGIL, &["test", &tests]),
        Invocation::new("<big.asm> --hex", SIGIL, &[&big, "--hex"]),
    ] {
        let (live, closed) = holds_to_the_rule(&inv);
        assert_eq!(live.status.code(), Some(0), "{}: the fixture must succeed", inv.what);
        assert!(closed.stderr.is_empty(), "{}: {}", inv.what, text(&closed.stderr));
    }
}

/// A run that fails keeps its failure when stdout is closed. Each of these three
/// reports the failure ON STDOUT, so the broken write lands on the failure itself:
/// `parse`'s diagnostics, `test`'s FAILED report, and the AS route's `message`
/// lines and failure line (its diagnostics go to stderr, which must arrive whole).
#[test]
fn a_failing_run_keeps_its_status_when_stdout_is_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bad = fixture(dir.path(), "bad.emp", "module m\nproc a( {\nproc b( {\nproc c( {\n");
    let failing = fixture(dir.path(), "failing.emp", FAILING_TESTS);
    let asm = fixture(
        dir.path(),
        "fail.asm",
        &format!(
            "{ASM_HEAD}\tmessage \"driver size is 42 bytes\"\n\tmessage \"a second line\"\n\
             \tdc.b nothing_defines_this\n\tend\n"
        ),
    );
    for inv in [
        Invocation::new("parse <bad.emp>", SIGIL, &["parse", &bad]),
        Invocation::new("test <failing.emp>", SIGIL, &["test", &failing]),
        Invocation::new("<fail.asm>", SIGIL, &[&asm]),
    ] {
        let (live, _closed) = holds_to_the_rule(&inv);
        assert_eq!(
            live.status.code(),
            Some(1),
            "{}: the fixture must fail with a live reader, or it is not the case under test. \
             stdout:\n{}",
            inv.what,
            text(&live.stdout)
        );
    }
}

/// The shape `| head` makes: the reader takes the first bytes and leaves while the
/// binary is still writing. Certain rather than raced, because the output is three
/// times `fs.pipe-max-size`: whatever the pipe holds when the reader leaves, the
/// binary still has bytes to write.
#[test]
fn a_reader_that_leaves_mid_stream_ends_the_run_as_it_would_have() {
    let dir = tempfile::tempdir().expect("tempdir");
    let big = big_image_asm(dir.path(), pipe_max_size() + 1);
    let mut child = Command::new(SIGIL)
        .args([big.as_str(), "--hex"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sigil");
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut head = [0u8; 4096];
    stdout.read_exact(&mut head).expect("the first 4096 bytes of the hex line");
    assert_eq!(&head[..6], b"AA AA ", "the reader is not reading the hex line");
    drop(stdout);
    let out = child.wait_with_output().expect("wait for sigil");
    assert_eq!(
        out.status.code(),
        Some(0),
        "the reader left mid-stream and the run ended {:?}; stderr:\n{}",
        out.status,
        text(&out.stderr)
    );
    assert!(out.stderr.is_empty(), "stderr:\n{}", text(&out.stderr));
}

/// With a live reader, every byte arrives. A rule that dropped every write would
/// pass every closed-stdout test above, so this is the half that tells it apart.
#[test]
fn a_live_reader_gets_every_byte() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Three times the largest pipe, so the line is written in many pieces. The hex
    // line is the image, not wording, so it is compared whole; the line after it is
    // held only to its `built: <n> bytes` prefix, its wording being free to change.
    let n = pipe_max_size() + 1;
    let big = big_image_asm(dir.path(), n);
    let out = Invocation::new("<big.asm> --hex", SIGIL, &[&big, "--hex"]).live();
    assert_eq!(out.status.code(), Some(0), "stderr:\n{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    let lines: Vec<&str> = stdout.split_inclusive('\n').collect();
    assert_eq!(lines.len(), 2, "the hex line and the built line, and nothing else");
    let hex_line = format!("{}\n", vec!["AA"; n].join(" "));
    assert!(lines[0] == hex_line, "the hex line arrived altered: {} bytes, expected {}", lines[0].len(), hex_line.len());
    assert!(
        lines[1].starts_with(&format!("built: {n} bytes")) && lines[1].ends_with('\n'),
        "the built line: {:?}",
        lines[1]
    );

    // The banner through a pipe is the banner through a file, whole: it ends on
    // one of its two closing sentences.
    let piped = Invocation::new("--version", SIGIL, &["--version"]).live();
    let file_path = dir.path().join("version.out");
    let file = std::fs::File::create(&file_path).expect("create version.out");
    let to_file = Command::new(SIGIL)
        .arg("--version")
        .stdout(Stdio::from(file))
        .output()
        .expect("spawn sigil");
    assert_eq!(to_file.status.code(), Some(0));
    let from_file = std::fs::read(&file_path).expect("read version.out");
    assert_eq!(text(&piped.stdout), text(&from_file), "a pipe and a file received different banners");
    let banner = text(&piped.stdout);
    assert!(
        banner.ends_with("both are positions.\n") || banner.ends_with("Do not treat it as current.\n"),
        "the banner does not end on its closing sentence, so part of it did not arrive:\n{banner}"
    );
}

/// Any stdout error but a broken pipe keeps std's handling: a full device fails
/// the run with std's message. A rule that ignored every write error would exit 0.
#[test]
fn a_full_device_on_stdout_still_fails_the_run() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("open /dev/full: this suite runs on Linux, where it exists");
    let out = Command::new(SIGIL)
        .arg("--help")
        .stdout(Stdio::from(full))
        .output()
        .expect("spawn sigil");
    let stderr = text(&out.stderr);
    assert_eq!(out.status.code(), Some(101), "a full device on stdout must fail the run; stderr:\n{stderr}");
    assert!(stderr.contains("failed printing to stdout: "), "std's message: {stderr}");
    assert!(!stderr.contains("Broken pipe"), "{stderr}");
}

/// Every tracked `.emp` under the workspace's `examples/`, as arguments.
fn example_emp_files() -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|e| e.expect("dir entry").path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "emp") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples"), &mut out);
    assert!(!out.is_empty(), "no .emp file under examples/, so emp_census had nothing to read");
    out.into_iter().map(|p| p.to_string_lossy().into_owned()).collect()
}

/// `emp_census` prints one TSV row per proc across every file it is given:
/// multi-line and unbounded, the output a reader cuts short with `head`.
#[test]
fn emp_census_into_a_closed_pipe_ends_as_it_would_have() {
    let files = example_emp_files();
    let args: Vec<&str> = files.iter().map(String::as_str).collect();
    let inv = Invocation::new("emp_census <examples>", EMP_CENSUS, &args);
    let (live, closed) = holds_to_the_rule(&inv);
    assert_eq!(closed.status.code(), Some(0), "stderr:\n{}", text(&closed.stderr));
    assert!(closed.stderr.is_empty(), "stderr:\n{}", text(&closed.stderr));
    let census = text(&live.stdout);
    assert!(census.starts_with("FILE\tLINE\tPROC\t"), "not the census header:\n{census}");
    assert!(census.lines().count() > 1, "the census found no proc, so it wrote one line:\n{census}");
}
