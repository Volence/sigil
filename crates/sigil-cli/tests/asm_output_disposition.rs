//! What `sigil <root.asm>` SAYS about its output versus what it DID with it.
//!
//! A run that assembles cleanly ends stdout on a `built:` line giving the byte
//! count and the DISPOSITION of the image: written to a named path, or not written
//! at all. Both routes print it through one function (`emit_image`), so this file
//! holds the AS route to the same gates `emp_output_disposition.rs` holds the emp
//! route to, and adds the properties only this route has a reason to pin:
//!
//! - the line is the LAST line of stdout, after the `--hex` line and after any
//!   `message` line, which makes it the success half of `fail_asm`'s failure line
//!   (`asm_failure_line.rs`): however a run ends, stdout's last line says how;
//! - it is on stdout and not stderr, because stderr is the diagnostic stream and
//!   `scripts/corpus-baseline.sh` reads a clean assembly off an empty one;
//! - a failed write still ends through `fail_asm`, not with a `built:` line.
//!
//! THE GATES ASSERT ON BEHAVIOUR, NOT ON A GOLDEN STRING, for the reason the emp
//! file gives: stdout is the thing under test, so it cannot also be the ground
//! truth. The wrote-case checks the file on disk, the discard-case checks the
//! directory, the `--hex` gate decodes its line against the artifact, and the
//! cross-route gate takes its expected line from `sigil emp` rather than from a
//! literal typed here. Every assembly runs across two shapes of different length,
//! so each claim is about the code path and not one fixture.

use std::path::Path;
use std::process::{Command, Output};

/// The AS header every fixture here starts with.
const HEADER: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";

/// A three-byte image.
const BODY_SHORT: &str = "\tdc.b 1,2,3\n\tend\n";

/// A longer image, so the pair of shapes disagrees on the byte count the success
/// line reports. Its exact length is never asserted, only that it differs from
/// `BODY_SHORT`'s and matches the artifact on disk.
const BODY_LONG: &str = "\tdc.b 1,2,3,4,5,6,7,8,9,10,11\n\tdc.w $1234\n\tend\n";

/// Run the bare `sigil` (the AS route) with `args` from `dir`.
fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run sigil")
}

/// Write `body` (after [`HEADER`]) to `dir/<stem>.asm` and return the file name.
fn write_source(dir: &Path, stem: &str, body: &str) -> String {
    let file = format!("{stem}.asm");
    std::fs::write(dir.join(&file), format!("{HEADER}{body}")).unwrap();
    file
}

/// A successful run's stdout lines, after asserting it succeeded and ended on its
/// `built:` line. Returning every line (not only the last) lets each gate state
/// what precedes the verdict as well as the verdict itself.
fn success_lines(out: &Output) -> Vec<String> {
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0; stdout: {stdout:?}, stderr: {stderr}");
    let lines: Vec<String> = stdout.lines().map(str::to_string).collect();
    assert!(
        lines.last().is_some_and(|l| l.starts_with("built: ")),
        "a succeeding run must end stdout on its `built:` line; stdout: {stdout:?}, \
         stderr: {stderr}"
    );
    lines
}

/// The `built:` line a successful run ended on.
fn built_line(out: &Output) -> String {
    success_lines(out).pop().expect("success_lines asserted a last line")
}

/// Parse the byte count out of a `built: N bytes...` line, so a gate's expectation
/// comes from the line itself and is then checked against the file.
fn reported_bytes(line: &str) -> usize {
    let rest = line.strip_prefix("built: ").expect("built: prefix");
    let n = rest.split_whitespace().next().expect("a count follows `built: `");
    n.parse().unwrap_or_else(|_| panic!("`{n}` is not a byte count in `{line}`"))
}

/// Decode a `--hex` line (`01 02 0A`) into bytes, refusing anything that is not
/// one.
fn decode_hex(line: &str) -> Vec<u8> {
    line.split(' ')
        .map(|t| {
            assert_eq!(t.len(), 2, "`{t}` is not a two-digit hex byte in `{line}`");
            u8::from_str_radix(t, 16).unwrap_or_else(|_| panic!("`{t}` is not hex in `{line}`"))
        })
        .collect()
}

/// The names in `dir`, sorted.
fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// One shape's half of the pair: assemble `body` twice from a fresh directory,
/// once without `-o` and once with it, and return the two `built:` lines plus the
/// length of the artifact the second run left on disk.
fn both_dispositions(body: &str, stem: &str) -> (String, String, usize) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let file = write_source(dir, stem, body);

    // Discard case: no `-o`. Nothing may appear in the directory.
    let discarded = built_line(&run(dir, &[&file]));
    assert_eq!(
        listing(dir),
        vec![file.clone()],
        "a run without -o must leave the directory holding only the source"
    );

    // Wrote case: `-o`. The artifact must exist and the line must name it.
    let out_name = format!("{stem}.bin");
    let wrote = built_line(&run(dir, &[&file, "-o", &out_name]));
    let artifact = dir.join(&out_name);
    assert!(artifact.exists(), "-o must leave an artifact on disk; line was `{wrote}`");
    let on_disk = std::fs::read(&artifact).unwrap().len();
    assert!(wrote.contains(&out_name), "the wrote-case line must name the path it wrote: `{wrote}`");
    assert_eq!(
        reported_bytes(&wrote),
        on_disk,
        "the reported count must be the artifact's real length; line was `{wrote}`"
    );
    assert_eq!(
        reported_bytes(&discarded),
        on_disk,
        "the discard-case count must be the length the same program writes; line was \
         `{discarded}`"
    );

    (discarded, wrote, on_disk)
}

/// The row this parcel closes. Before it, a clean `sigil <root.asm>` printed NOTHING
/// whether it wrote a file or not, so a run that left a ROM on disk, a run that
/// threw the image away, and a command that never started all looked alike. A
/// passing run here MUST have failed on the `built:` line assertion under that
/// binary.
///
/// Building without `-o` stays exit 0 (the helper asserts it): a syntax-check run
/// is legitimate, and the honest line must not have been bought by making it an
/// error.
#[test]
fn built_line_distinguishes_written_from_discarded() {
    let (short_discard, short_wrote, short_len) = both_dispositions(BODY_SHORT, "short");
    let (long_discard, long_wrote, long_len) = both_dispositions(BODY_LONG, "long");

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

/// `--hex` prints the image as one line and the `built:` line FOLLOWS it, after any
/// `message` line the program printed first. The verdict is last on success as
/// `fail_asm`'s is on failure, so `sigil root.asm --hex > log` ends on how the run
/// went either way.
///
/// The hex line is decoded and compared with the artifact the same run wrote, so
/// the gate knows it found the image and not some other line of stdout. Without
/// `-o` the run still exits 0 and still reports the build.
#[test]
fn hex_line_precedes_the_built_line_which_ends_stdout() {
    for (stem, body) in [("short", BODY_SHORT), ("long", BODY_LONG)] {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let file = write_source(dir, stem, &format!("\tmessage \"size \\{{7}}\"\n{body}"));
        let out_name = format!("{stem}.bin");

        let wrote = success_lines(&run(dir, &[&file, "--hex", "-o", &out_name]));
        let image = std::fs::read(dir.join(&out_name)).expect("-o must write the artifact");
        assert_eq!(
            wrote.len(),
            3,
            "stdout must hold the message, the hex line and the built line: {wrote:?}"
        );
        assert_eq!(wrote[0], "size 7", "the program's message leads stdout: {wrote:?}");
        assert_eq!(decode_hex(&wrote[1]), image, "the hex line is the image on disk: {wrote:?}");
        assert!(wrote[2].contains(&out_name), "and the built line names the file: {wrote:?}");
        assert_eq!(reported_bytes(&wrote[2]), image.len(), "at its real length: {wrote:?}");

        std::fs::remove_file(dir.join(&out_name)).unwrap();
        let discarded = success_lines(&run(dir, &[&file, "--hex"]));
        assert_eq!(
            discarded.len(),
            3,
            "a --hex run without -o prints the same three lines: {discarded:?}"
        );
        assert_eq!(decode_hex(&discarded[1]), image, "and the same image: {discarded:?}");
        assert!(
            discarded[2].contains("no file written"),
            "and says it wrote nothing: {discarded:?}"
        );
        assert!(!dir.join(&out_name).exists(), "and writes nothing");
    }
}

/// The success line is on STDOUT and a clean run leaves stderr EMPTY.
/// `scripts/corpus-baseline.sh` counts stderr's lines as the diagnostic population
/// and calls exit 0 over an empty stream a clean assembly, so a success line there
/// would count as a diagnostic on every clean run.
#[test]
fn a_clean_run_reports_on_stdout_and_leaves_stderr_empty() {
    for (stem, body) in [("short", BODY_SHORT), ("long", BODY_LONG)] {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let file = write_source(dir, stem, body);
        let out_name = format!("{stem}.bin");
        for args in [vec![file.as_str()], vec![file.as_str(), "-o", out_name.as_str()]] {
            let out = run(dir, &args);
            built_line(&out);
            assert_eq!(
                String::from_utf8_lossy(&out.stderr),
                "",
                "a clean `sigil {args:?}` must write nothing to stderr"
            );
        }
    }
}

/// The two routes print ONE line for ONE outcome. An AS program and an emp module
/// that assemble to the same four bytes, run the same way, must report their
/// builds identically; the expected line is whatever `sigil emp` prints, never a
/// literal copied here. The artifacts are compared too, so the equality of the
/// lines is a fact about the same outcome and not a coincidence of two counts.
#[test]
fn the_as_and_emp_routes_print_one_line_for_one_outcome() {
    let as_tmp = tempfile::tempdir().unwrap();
    let emp_tmp = tempfile::tempdir().unwrap();
    let (as_dir, emp_dir) = (as_tmp.path(), emp_tmp.path());
    let asm = write_source(as_dir, "prog", "\tmoveq #1,d0\n\trts\n\tend\n");
    std::fs::write(
        emp_dir.join("prog.emp"),
        "module prog\nproc entry () {\n    moveq #1, d0\n    rts\n}\n",
    )
    .unwrap();

    let as_wrote = built_line(&run(as_dir, &[&asm, "-o", "prog.bin"]));
    let emp_wrote = built_line(&run(emp_dir, &["emp", "prog.emp", "-o", "prog.bin"]));
    assert_eq!(
        std::fs::read(as_dir.join("prog.bin")).unwrap(),
        std::fs::read(emp_dir.join("prog.bin")).unwrap(),
        "the two fixtures must build the same image, or the lines are about different \
         outcomes"
    );
    assert_eq!(as_wrote, emp_wrote, "a written image is reported alike on both routes");

    let as_discard = built_line(&run(as_dir, &[&asm]));
    let emp_discard = built_line(&run(emp_dir, &["emp", "prog.emp"]));
    assert_eq!(as_discard, emp_discard, "a discarded image is reported alike on both routes");
}

/// A write that FAILS is a failed run, not a build: exit 1, no `built:` line, and
/// `fail_asm`'s line last on stdout. The shared tail reports the write error and
/// returns it rather than exiting, so this is the gate that the AS route still
/// ends such a run through `fail_asm`.
#[test]
fn a_failed_write_ends_through_fail_asm_and_reports_no_build() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let file = write_source(dir, "short", BODY_SHORT);
    let out = run(dir, &[&file, "-o", "no_such_dir/short.bin"]);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

    assert_eq!(out.status.code(), Some(1), "stdout: {stdout:?}, stderr: {stderr}");
    assert!(stderr.contains("cannot write"), "the write error is the reported one: {stderr}");
    assert!(!stdout.contains("built:"), "a failed write must not report a build: {stdout:?}");
    assert_eq!(
        stdout.lines().next_back(),
        Some("assembly failed: 1 error (reported on stderr)"),
        "the failure line must end stdout: {stdout:?}"
    );
}
