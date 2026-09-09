//! What `sigil emp` SAYS about its output versus what it DID with it.
//!
//! Two properties, one subject. The success line must state the disposition of the
//! image (written to a named path, or discarded), and a flag the chosen code path
//! cannot honour must be refused by name rather than consumed and dropped. Both are
//! failures of the same kind: the run reports success in a wording that is equally
//! true of the outcome the caller wanted and the one they did not get.
//!
//! THE GATES ASSERT ON BEHAVIOUR, NOT ON A GOLDEN STRING. Stdout is the thing under
//! test, so it cannot also be the ground truth. The wrote-case checks the FILE: it
//! exists, its length on disk equals the count the line printed, and the path the
//! line names is the path that exists. The discard-case checks that the directory
//! gained nothing.
//!
//! AND THEY ASSERT ACROSS TWO SHAPES OF DIFFERENT LENGTH, deliberately. A single
//! input can only establish "this file behaves this way"; two inputs whose byte
//! counts differ establish it of the CODE PATH, which is what is being claimed. The
//! expected length is read back from the artifact rather than copied from a
//! measurement, so a legitimate change in what these fixtures lower to cannot make
//! the gate lie.

use std::path::Path;
use std::process::{Command, Output};

/// A module that lowers to a `moveq`/`rts` pair (4 bytes).
const SRC_SHORT: &str = "module short\nproc entry () {\n    moveq #1, d0\n    rts\n}\n";

/// A longer module, so the pair of shapes disagrees on the byte count the success
/// line reports. Its exact length is never asserted, only that it differs from
/// `SRC_SHORT`'s and matches the artifact on disk.
const SRC_LONG: &str = "module long\nproc entry () {\n    moveq #1, d0\n    moveq #2, d1\n    \
                        moveq #3, d2\n    moveq #4, d3\n    moveq #5, d4\n    rts\n}\n";

/// Run `sigil emp` with `args` from `dir`.
fn run(dir: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.arg("emp");
    cmd.args(args);
    cmd.current_dir(dir);
    cmd.output().expect("run sigil emp")
}

/// The `built: N bytes...` line out of a successful run's stdout.
fn built_line(out: &Output) -> String {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0; stdout: {stdout}, stderr: {stderr}");
    stdout
        .lines()
        .find(|l| l.starts_with("built:"))
        .unwrap_or_else(|| panic!("no `built:` line; stdout: {stdout}, stderr: {stderr}"))
        .to_string()
}

/// Parse the byte count out of a `built: N bytes...` line, so the gate's expectation
/// comes from the line itself and is then checked against the file rather than
/// against a number typed here.
fn reported_bytes(line: &str) -> usize {
    let rest = line.strip_prefix("built: ").expect("built: prefix");
    let n = rest.split_whitespace().next().expect("a count follows `built: `");
    n.parse().unwrap_or_else(|_| panic!("`{n}` is not a byte count in `{line}`"))
}

/// One shape's half of the pair: compile `src` twice from a fresh directory, once
/// discarding the image and once writing it, and return the two `built:` lines plus
/// the length of the artifact that the second run left on disk.
fn both_dispositions(src: &str, stem: &str) -> (String, String, usize) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let file = format!("{stem}.emp");
    std::fs::write(dir.join(&file), src).unwrap();

    // Discard case: no `-o`. Nothing may appear in the directory.
    let discarded = built_line(&run(dir, &[&file]));
    let after: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        after,
        vec![file.clone()],
        "a run without -o must leave the directory holding only the source"
    );

    // Wrote case: `-o`. The artifact must exist and the line must name it.
    let out_name = format!("{stem}.bin");
    let wrote = built_line(&run(dir, &[&file, "-o", &out_name]));
    let artifact = dir.join(&out_name);
    assert!(artifact.exists(), "-o must leave an artifact on disk; line was `{wrote}`");
    let on_disk = std::fs::read(&artifact).unwrap().len();
    assert!(
        wrote.contains(&out_name),
        "the wrote-case line must name the path it wrote: `{wrote}`"
    );
    assert_eq!(
        reported_bytes(&wrote),
        on_disk,
        "the reported count must be the artifact's real length; line was `{wrote}`"
    );

    (discarded, wrote, on_disk)
}

/// The row this parcel closes. Before the fix these two runs printed a
/// BYTE-IDENTICAL line at exit 0 while one wrote a ROM and the other threw it away,
/// so a reader had no way to tell a fresh artifact from a stale one they were about
/// to trust. A passing run here MUST have failed on `distinct` under that binary.
///
/// The two shapes are the point: `short` and `long` lower to different lengths, so
/// the property being asserted is the code path's, not one fixture's.
#[test]
fn built_line_distinguishes_written_from_discarded() {
    let (short_discard, short_wrote, short_len) = both_dispositions(SRC_SHORT, "short");
    let (long_discard, long_wrote, long_len) = both_dispositions(SRC_LONG, "long");

    assert_ne!(
        short_len, long_len,
        "the two shapes must differ in length, or this is one measurement twice \
         (short={short_len}, long={long_len})"
    );

    for (discard, wrote) in [(&short_discard, &short_wrote), (&long_discard, &long_wrote)] {
        assert_ne!(
            discard, wrote,
            "the success line must distinguish a written image from a discarded one; \
             both runs printed `{discard}`"
        );
        assert!(
            discard.contains("no file written"),
            "the discard-case line must say nothing was written: `{discard}`"
        );
        assert!(
            discard.contains("-o"),
            "the discard-case line must name the flag that would write one: `{discard}`"
        );
    }
}

/// Building without `-o` stays a SUCCESS. `--hex` and a plain syntax check are
/// legitimate uses, so the honest line must not have been bought by turning the
/// no-output case into an error. Without this the gate above is satisfied by a
/// compiler that refuses half of its own contract.
#[test]
fn discarding_the_image_is_not_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    std::fs::write(dir.join("short.emp"), SRC_SHORT).unwrap();

    let out = run(dir, &["short.emp", "--hex"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "a `--hex` run with no -o must exit 0; stdout: {stdout}");
    assert!(stdout.contains("built:"), "and must still report the build: {stdout}");
}

/// `--prelude` on the single-file path is refused BY NAME, at the same exit code its
/// sibling `--map` uses for the same class of mistake. Before the fix the flag was
/// parsed, consumed and dropped: the run was indistinguishable from not passing it
/// at all, including when its argument named nothing that exists. A passing run MUST
/// have failed on the exit-code assertion under that binary, which returned 0.
#[test]
fn prelude_without_root_is_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    std::fs::write(dir.join("short.emp"), SRC_SHORT).unwrap();

    let out = run(dir, &["short.emp", "--prelude", "no_such_module"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        out.status.code(),
        Some(2),
        "--prelude without --root is a usage error (exit 2, as --map is); \
         stdout: {stdout}, stderr: {stderr}"
    );
    assert!(stderr.contains("--prelude"), "the refusal must name the flag: {stderr}");
    assert!(stderr.contains("--root"), "and state the precondition: {stderr}");
    assert!(
        stderr.contains("pass --root") || stderr.contains("drop --prelude"),
        "and say what to do about it: {stderr}"
    );
    assert!(
        !stdout.contains("built:"),
        "a refused invocation must not report a build: {stdout}"
    );
}

/// The exit code above is DERIVED from its sibling rather than copied from a
/// measurement: `--map` without `--root` is the same class of mistake and this
/// pins the two together, so a future change to one that leaves the other behind
/// shows up here.
#[test]
fn map_and_prelude_refuse_at_the_same_exit_code() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    std::fs::write(dir.join("short.emp"), SRC_SHORT).unwrap();

    let map = run(dir, &["short.emp", "--map", "regions.toml"]);
    let prelude = run(dir, &["short.emp", "--prelude", "no_such_module"]);

    assert_eq!(
        map.status.code(),
        prelude.status.code(),
        "--map and --prelude are the same class of usage error and must exit alike; \
         --map: {:?} / --prelude: {:?}",
        map.status.code(),
        prelude.status.code()
    );
    let map_err = String::from_utf8_lossy(&map.stderr);
    assert!(map_err.contains("--map"), "the --map refusal must name its flag: {map_err}");
    assert!(
        map_err.contains("pass --root") || map_err.contains("drop --map"),
        "and say what to do about it: {map_err}"
    );
}

/// Write a two-module program (a prelude exporting `K`, an entry consuming it) into
/// `dir`, so a `--root`/`--prelude` build has something real to resolve.
fn write_prelude_program(dir: &Path) {
    std::fs::write(dir.join("prelude.emp"), "module prelude\npub const K = 1\n").unwrap();
    std::fs::write(
        dir.join("entry.emp"),
        "module entry\nproc go () {\n    moveq #K, d0\n    rts\n}\n",
    )
    .unwrap();
}

/// CONTROL for the refusal: `--prelude` WITH `--root` still resolves a real prelude
/// module and still builds. Without this arm the gate above is satisfied by a
/// binary that has simply stopped accepting the flag.
///
/// It asserts ONLY the control property, which means it must stay GREEN when the
/// refusal is reverted. An assertion about the wording of the success line does not
/// belong here: a control that also fails for the reason the subject fails cannot
/// tell the two apart, and is not a control. That claim is
/// `built_line_names_the_path_on_the_root_path`, below.
#[test]
fn prelude_with_root_still_builds() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_prelude_program(dir);

    let out = run(dir, &["entry.emp", "--root", ".", "--prelude", "prelude", "-o", "entry.bin"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "a real prelude under --root must build; stderr: {stderr}");
    assert!(dir.join("entry.bin").exists(), "and write its artifact; stderr: {stderr}");
}

/// The `--root` path reaches the success line through its own caller
/// (`run_emp_program`), so a fix proven on the single-file path is not yet a fact
/// about this one. Same behavioural assertions: the named path is the path that
/// exists, and the reported count is the artifact's real length.
#[test]
fn built_line_names_the_path_on_the_root_path() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_prelude_program(dir);

    let out = run(dir, &["entry.emp", "--root", ".", "--prelude", "prelude", "-o", "entry.bin"]);
    let line = built_line(&out);
    let artifact = dir.join("entry.bin");
    assert!(artifact.exists(), "the multi-module path must write its artifact; line: `{line}`");
    assert!(line.contains("entry.bin"), "and name it in the success line: `{line}`");
    assert_eq!(
        reported_bytes(&line),
        std::fs::read(&artifact).unwrap().len(),
        "and report the artifact's real length; line: `{line}`"
    );
}
