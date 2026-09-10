//! A failing `sigil <root.asm>` run says which stages did NOT run.
//!
//! UXa F5, ledger row `sig-ux-partial-error-list`: the error list is silently
//! partial across phase boundaries. Seven errors present, five reported, two
//! withheld until the first five were fixed, with no count and no note. Each
//! hidden error costs a full build cycle to discover.
//!
//! Owner ruling `d-28` answered `report_everything`, and the half that landed at
//! `451cb3e2` removed the front end's cautious deferral pass. That is a
//! different mechanism from the one this file is about: the deferral pass lives
//! entirely inside stage 1, so removing it made stage 1 report more and could
//! not move a diagnostic across a stage boundary. `d-28-answered` left the third
//! option's other half ("the assembler says plainly when it is holding errors
//! back") as implementation strategy under `d-2`, conditional on measurement
//! after the first half showing errors still withheld at a stage boundary. The
//! measurement is in
//! `docs/superpowers/notes/2026-09-10-ux-partial-error-list-repro/README.md`,
//! and it shows exactly that.
//!
//! **These tests run the COMMITTED probe files rather than writing their own
//! copies into a temp directory, and that is the point.** The row could not be
//! closed for a month because `451cb3e2` cited its witness as
//! `.scratch/repro/control.asm`, a path in a per-worktree scratch area that no
//! longer exists, so the reproduction could not be re-run by anyone. Reading the
//! tracked files here means the witness cannot go missing again without this
//! file going red, and it means these assertions are checked against the same
//! bytes a person reads in the note.
//!
//! What is NOT claimed anywhere here: that the run could continue past a failed
//! stage. It cannot. The stages are not independent checks over one input; each
//! consumes the previous stage's success value, so the front end returns a
//! `Failure` carrying no `Module` and `resolve_layout` returns diagnostics
//! carrying no sections. The remedy is to stop silence being mistaken for
//! completeness, not to abolish the boundary.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test, built by cargo for this integration target.
const SIGIL: &str = env!("CARGO_BIN_EXE_sigil");

/// The tracked probe directory, resolved from this crate's manifest rather than
/// from the working directory, so the tests do not depend on where cargo was
/// invoked from.
fn probes() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/sigil-cli has a repository root two levels up")
        .join("docs/superpowers/notes/2026-09-10-ux-partial-error-list-repro");
    assert!(
        dir.is_dir(),
        "the F5 probe directory is missing at {}. It is the witness this row was \
         closed against and it is tracked on purpose, because the previous witness \
         lived under .scratch/ and vanished with its worktree. Do not paper over \
         this by inlining the sources here.",
        dir.display()
    );
    dir
}

/// Assemble one tracked probe, from its own directory, discarding the image.
///
/// Every probe in that directory is expected to FAIL before the output stage, so
/// `-o /dev/null` is never reached and cannot mask a result.
fn run(probe: &str) -> Output {
    let dir = probes();
    let path = dir.join(probe);
    assert!(path.is_file(), "probe {} is missing", path.display());
    Command::new(SIGIL)
        .current_dir(&dir)
        .arg(probe)
        .arg("-o")
        .arg("/dev/null")
        .output()
        .expect("spawn sigil")
}

/// The `error:` lines on stderr. This is the list a person reads.
fn error_lines(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.contains("error:"))
        .map(str::to_string)
        .collect()
}

/// The incompleteness caveat on stdout, if the run printed one.
fn caveat(out: &Output) -> Option<String> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with("this error list may be incomplete:"))
        .map(str::to_string)
}

/// The last line of stdout.
fn last_stdout_line(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next_back()
        .unwrap_or("")
        .to_string()
}

/// **The finding itself, re-measured.** `many.asm` holds seven errors. Five are
/// front-end errors and two are link errors, and the run reports the five.
///
/// The five and the two are not asserted as literals: `many2.asm` is `many.asm`
/// with exactly those five repaired and nothing else touched, so the two it
/// reports ARE the withheld pair, derived by running it rather than copied from
/// the seat's transcript. A test that only counted `many.asm`'s five could not
/// tell a partial list from a complete one.
#[test]
fn the_seven_five_two_repro_still_measures_seven_five_two() {
    let partial = run("many.asm");
    let rest = run("many2.asm");
    assert_eq!(partial.status.code(), Some(1));
    assert_eq!(rest.status.code(), Some(1));

    let reported = error_lines(&partial);
    let withheld = error_lines(&rest);
    assert_eq!(
        reported.len(),
        5,
        "many.asm must report five front-end errors, got {reported:#?}"
    );
    assert_eq!(
        withheld.len(),
        2,
        "many2.asm must report the two link errors many.asm withheld, got {withheld:#?}"
    );

    // The withheld pair is a LINK-stage class, which is what makes this a stage
    // boundary and not a second front-end pass.
    for line in &withheld {
        assert!(
            line.contains("for fixup in section"),
            "the withheld errors must be link fixup errors, got {line:?}"
        );
    }
    // And none of them appears in the partial list, which is the withholding.
    for line in &withheld {
        let symbol = line.split('`').nth(1).expect("the diagnostic names a symbol");
        assert!(
            !reported.iter().any(|r| r.contains(symbol)),
            "many.asm reported {symbol}, so nothing was withheld and this test has \
             stopped measuring its subject: {reported:#?}"
        );
    }

    let note = caveat(&partial).expect(
        "many.asm withholds two errors, so the run must say the later stages did not run",
    );
    assert_eq!(
        note,
        "this error list may be incomplete: sigil stopped at the front end, so layout, \
         link and the image checks did not run"
    );

    // `many2.asm` reaches LINK and stops there, so it pins the third stage's
    // wording. Without this the link arm's text is the one arm no test reads,
    // and a wrong stage list there would ship green.
    assert_eq!(
        caveat(&rest).expect("many2.asm stops at link, so the image checks do not run"),
        "this error list may be incomplete: sigil stopped at link, so the image checks \
         did not run"
    );
}

/// The layout to link boundary, which is the shape `s1disasm` is in: a clean
/// front end that stops at layout. `451cb3e2` did not touch this boundary and
/// this is the case that proves the row was not already closed by it.
#[test]
fn a_layout_failure_says_link_did_not_run() {
    let partial = run("layout-then-link.asm");
    let rest = run("layout-then-link-fixed.asm");

    let reported = error_lines(&partial);
    let withheld = error_lines(&rest);
    assert_eq!(reported.len(), 1, "got {reported:#?}");
    assert_eq!(withheld.len(), 2, "got {withheld:#?}");
    assert!(
        reported[0].contains("unresolved jmp/jsr target"),
        "the reported error must be the layout-stage one: {reported:#?}"
    );

    assert_eq!(
        caveat(&partial).expect("a layout failure withholds link's diagnostics"),
        "this error list may be incomplete: sigil stopped at layout, so link and the \
         image checks did not run"
    );
}

/// **The positive control on the boundary that WAS fixed, and the reason a zero
/// here is worth believing.**
///
/// `frontend-then-layout.asm` holds a front-end error and an unresolved
/// `jmp` target. Before `451cb3e2` the front end deferred that target
/// symbolically and left it to layout, so the front-end error hid it and the run
/// printed ONE error. It now prints both in one run. If this test ever reports
/// one error again, the d-28 landing has been reverted and the other two tests
/// in this file are measuring a different program.
#[test]
fn the_boundary_d28_fixed_reports_both_errors_in_one_run() {
    let out = run("frontend-then-layout.asm");
    let reported = error_lines(&out);
    assert_eq!(
        reported.len(),
        2,
        "d-28 landed at 451cb3e2 so both errors must appear in one run, got {reported:#?}"
    );
    assert!(reported[0].contains("not a recognized 68000 mnemonic"), "{reported:#?}");
    assert!(reported[1].contains("unresolved symbol `NoSuchTarget` in operand"), "{reported:#?}");
}

/// **The negative control on the caveat.** A run that reaches the output stage
/// is holding nothing back and must not say that it is. Without this, a caveat
/// printed unconditionally would satisfy every other assertion in this file and
/// mean nothing.
///
/// `-o /dev/null` is the probe: the atomic installer renames into the output's
/// directory, which for `/dev/null` is `/dev`, so a SUCCEEDING assembly fails at
/// the output stage. That is `Stage::Image`, the one variant whose
/// `stages_not_run` is `None`.
#[test]
fn a_failure_at_the_output_stage_claims_nothing_is_withheld() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("clean.asm");
    std::fs::write(&src, "\tcpu 68000\n\tpadding off\n\torg 0\n\tnop\n").expect("write");
    let out = Command::new(SIGIL)
        .arg(&src)
        .arg("-o")
        .arg("/dev/null")
        .output()
        .expect("spawn sigil");
    assert_eq!(
        out.status.code(),
        Some(1),
        "this control needs a run that assembles cleanly and then fails to install; \
         if installing to /dev/null started working, replace the probe rather than \
         deleting the control. stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("cannot write"),
        "the failure must be the output stage, not an earlier one: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        caveat(&out),
        None,
        "nothing was withheld here, so the run must not say the list may be incomplete"
    );
}

/// A run that succeeds says nothing at all. The second half of the control
/// above: the caveat is a property of failing at an early stage, not of running.
#[test]
fn a_succeeding_run_says_nothing_about_withheld_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("clean.asm");
    let bin = dir.path().join("clean.bin");
    std::fs::write(&src, "\tcpu 68000\n\tpadding off\n\torg 0\n\tnop\n").expect("write");
    let out = Command::new(SIGIL)
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("spawn sigil");
    assert_eq!(out.status.code(), Some(0), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
}

/// The caveat goes BEFORE the failure line, so `assembly failed: N errors` stays
/// the last thing on stdout.
///
/// That is the UXa F8 property (`asm_failure_line.rs` pins it by reading the
/// last line of stdout), and this file must not quietly take it away while
/// closing a neighbouring finding.
#[test]
fn the_failure_line_is_still_the_last_line_of_stdout() {
    let out = run("many.asm");
    assert_eq!(last_stdout_line(&out), "assembly failed: 5 errors (reported on stderr)");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let note_at = stdout.find("this error list may be incomplete").expect("the caveat prints");
    let fail_at = stdout.find("assembly failed:").expect("the failure line prints");
    assert!(note_at < fail_at, "the caveat must precede the failure line: {stdout:?}");
}

/// **`run_asm` reaches its failure exits in the order the enum declares.**
///
/// This is the assumption every other test in this file rests on and none of
/// them can see. The caveat's whole content is "the stages AFTER this one did
/// not run", which is true only if the stage a call site names is where that
/// call site actually sits in the sequence. Nothing about a `Stage` argument is
/// checked by the compiler: passing `Stage::Link` at the layout exit compiles,
/// prints a confidently wrong list, and leaves the behavioural tests here green
/// for every boundary they do not happen to probe.
///
/// Stated over the source because the alternative is a probe per call site, and
/// two of the four stages cannot be reached by a probe that also reaches the
/// others. The expectation is DERIVED: both the declaration order and the call
/// order are read out of `main.rs`, and the test asserts the relationship
/// between them rather than a copy of either.
#[test]
fn run_asm_reaches_its_stages_in_the_order_the_enum_declares_them() {
    let variants = stage_variants();
    let body = run_asm_body();
    let calls: Vec<usize> = body
        .match_indices("Stage::")
        .map(|(i, _)| {
            let rest = &body[i + "Stage::".len()..];
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
            variants
                .iter()
                .position(|v| *v == name)
                .unwrap_or_else(|| panic!("run_asm names Stage::{name}, which the enum does not declare"))
        })
        .collect();
    assert!(
        calls.len() >= 6,
        "run_asm names {} stages, fewer than its six failure exits; the slice has \
         stopped seeing the body",
        calls.len()
    );
    assert!(
        calls.windows(2).all(|w| w[0] <= w[1]),
        "run_asm's failure exits name stages out of declaration order: {calls:?} against \
         {variants:?}. A call site that names a stage other than its own position prints \
         a confidently wrong list of what did not run."
    );
    assert_eq!(
        calls.first().copied(),
        Some(0),
        "the first failure exit must be the first stage"
    );
    assert_eq!(
        calls.last().copied(),
        Some(variants.len() - 1),
        "the last failure exit must be the last stage"
    );
}

/// `Stage`'s variants, in declaration order, read out of `main.rs`.
fn stage_variants() -> Vec<String> {
    const SOURCE: &str = include_str!("../src/main.rs");
    let start = SOURCE.find("enum Stage {").expect("main.rs declares enum Stage");
    let body = &SOURCE[start..];
    let end = body.find("\n}\n").expect("enum Stage closes at column zero");
    let variants: Vec<String> = body[..end]
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            t.strip_suffix(',')
                .filter(|v| {
                    v.chars().next().is_some_and(char::is_uppercase)
                        && v.chars().all(char::is_alphanumeric)
                })
                .map(str::to_string)
        })
        .collect();
    assert!(
        variants.len() >= 4,
        "parsed {} Stage variants, which is fewer than the four these gates were written \
         over; the slice has stopped seeing the enum: {variants:?}",
        variants.len()
    );
    variants
}

/// The body of `run_asm` as `main.rs` declares it, from its `fn` line to the
/// closing brace at column zero. Same device as `asm_failure_line.rs`, and it
/// asserts its own shape so a slice that came back empty cannot make a gate
/// green for the wrong reason.
fn run_asm_body() -> &'static str {
    const SOURCE: &str = include_str!("../src/main.rs");
    let start = SOURCE.find("\nfn run_asm(").expect("main.rs declares fn run_asm");
    let rest = &SOURCE[start + 1..];
    let end = rest.find("\n}\n").expect("fn run_asm closes at column zero") + 3;
    let body = &rest[..end];
    assert!(
        body.contains("assemble_root_located_warned"),
        "the slice is not run_asm's body"
    );
    body
}

/// Every stage that can hold a later stage's diagnostics names the stages it
/// skipped, and the last one names none. Read off `main.rs` so a stage added
/// later cannot be given a `stages_not_run` arm and no note by accident.
///
/// The expectation is DERIVED: the variant list and the arms are both read out
/// of the source, and the test asserts the relationship between them rather than
/// a copy of either.
#[test]
fn every_stage_but_the_last_names_what_it_skipped() {
    const SOURCE: &str = include_str!("../src/main.rs");
    let variants = stage_variants();

    let arms_at = SOURCE.find("fn stages_not_run(").expect("Stage has stages_not_run");
    let arms = &SOURCE[arms_at..];
    let arms_end = arms.find("\n    }\n").expect("stages_not_run closes");
    let arms = &arms[..arms_end];
    for v in &variants {
        assert!(
            arms.contains(&format!("Stage::{v} =>")),
            "Stage::{v} has no stages_not_run arm, so a run stopping there would say \
             nothing about the stages it skipped"
        );
    }
    let nones = arms.matches("=> None").count();
    assert_eq!(
        nones, 1,
        "exactly one stage, the last, must report nothing withheld. {nones} arms return \
         None, so either a stage that DOES withhold has gone silent or the caveat has \
         become unconditional and stopped meaning anything."
    );
}
