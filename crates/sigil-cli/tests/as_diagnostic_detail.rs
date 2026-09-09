//! What an AS-surface diagnostic tells a person: which character, which
//! column, and which operand.
//!
//! Three defects, found together by the UXb seat and closed together, but they
//! are three and the middle one does not imply the last:
//!
//! 1. `error: unexpected character` named neither the character nor a column.
//! 2. `SourceMap::label` computed a column and threw it away, so the AS
//!    surface reported less than the same binary's `.emp` surface from the
//!    same data.
//! 3. Two operands on one line rendered byte-identically, because every item
//!    of a data directive was blamed at the DIRECTIVE's span. Adding the
//!    column did NOT fix this on its own: `dc.b Big, Big` still printed the
//!    same column twice. The item's own span had to move too.
//!
//! The two diagnostic DIALECTS are deliberate and are not a defect. `.emp`
//! renders `path:line:col:` and the AS surface renders `file(line):col:`;
//! docs/OVERSEER.md rules the split on the argument that a compatibility
//! surface's job is to be the thing it is compatible with. The gate at the end
//! holds the split open in BOTH directions, so neither one can later be
//! "fixed" into the other by someone who cannot tell which was intended.
//!
//! `file(line):col:` is asl's OWN spelling, measured and not chosen: on the
//! reference build asl reports `h.asm(2):9: error #1010: symbol undefined`,
//! and across five assignment spellings its numbers are the 1-based columns of
//! the offending token. The first draft of this fix invented `file(line,col)`
//! from the Microsoft convention without checking, which would have been a
//! third dialect answering a question the incumbent had already answered.

use std::process::{Command, Output};

/// The binary under test, built by cargo for this integration target.
const SIGIL: &str = env!("CARGO_BIN_EXE_sigil");

/// Assemble `body` (after a `cpu 68000` line) from a fresh directory, and hand
/// back the run together with the file's name as diagnostics will spell it.
fn run(body: &str) -> (Output, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("\tcpu 68000\n\tpadding off\n\torg 0\n{body}")).expect("write");
    let out = Command::new(SIGIL).arg(&path).output().expect("spawn sigil");
    (out, path.to_string_lossy().into_owned())
}

/// Every stderr line of `out` that carries a diagnostic.
fn diags(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.contains("error:") || l.contains("warning:"))
        .map(str::to_string)
        .collect()
}

/// A stray character is named, and located at its own column.
///
/// The column is derived here rather than pinned: the probe puts the backtick
/// at a position this test computes from the line it wrote, so a change to the
/// probe cannot leave a stale literal passing.
#[test]
fn an_unexpected_character_is_named_and_located() {
    let line = "\tmove.w\t#`x, d0";
    let (out, _) = run(&format!("{line}\n\trts\n"));
    assert_eq!(out.status.code(), Some(1));
    let d = diags(&out);
    assert_eq!(d.len(), 1, "one diagnostic expected: {d:?}");
    // Columns count characters from 1, and the probe's line is all ASCII.
    let col = line.find('`').expect("the probe writes a backtick") + 1;
    assert!(
        d[0].contains("unexpected character '`'"),
        "the character must be named: {:?}",
        d[0]
    );
    assert!(
        d[0].contains(&format!("(4):{col}:")),
        "expected line 4 column {col} (the probe's own backtick position): {:?}",
        d[0]
    );
}

/// A non-ASCII stray character is reported as the character the author typed,
/// not as the first byte of its UTF-8 encoding, and the column counts
/// characters.
#[test]
fn a_non_ascii_unexpected_character_is_reported_as_one_character() {
    let (out, _) = run("\tmove.w\t#\u{a3}, d0\n\trts\n");
    let d = diags(&out);
    assert_eq!(d.len(), 1, "one diagnostic expected: {d:?}");
    assert!(
        d[0].contains("unexpected character '\u{a3}'"),
        "the character must survive as itself: {:?}",
        d[0]
    );
}

/// The AS surface carries a column at all. The `.emp` front end on the SAME
/// binary already did, which is what made this a discarded fact rather than a
/// missing one.
///
/// The assertion reaches PAST the `(4):`, because the pre-fix format was
/// `probe.asm(4): error: …` and `starts_with("probe.asm(4):")` is true of it.
/// A gate for "there is a column" that the columnless format satisfies is the
/// gate this row exists to complain about.
#[test]
fn an_as_diagnostic_carries_a_column() {
    let (out, name) = run("\tdc.b nothing_defines_this\n\tend\n");
    let d = diags(&out);
    assert_eq!(d.len(), 1, "one diagnostic expected: {d:?}");
    let head = format!("{name}(4):");
    let rest = d[0]
        .strip_prefix(&head)
        .unwrap_or_else(|| panic!("expected `{head}<col>: …`, got {:?}", d[0]));
    let col = rest.split(':').next().unwrap_or("");
    assert!(
        col.parse::<u32>().is_ok_and(|n| n >= 1),
        "the field after the line number must be a 1-based column, got {col:?} in {:?}",
        d[0]
    );
}

/// **The row's own exhibit.** Two items of one directive are two diagnostics
/// at two DIFFERENT columns, and those columns are the items' own positions.
///
/// Before the fix these two lines were byte-identical and a reader could not
/// tell which item either was about.
#[test]
fn two_operands_on_one_line_are_told_apart_by_their_columns() {
    let line = "\tdc.b\tBig, Big";
    let (out, _) = run(&format!("Big:\tequ\t$12345\n{line}\n\trts\n"));
    assert_eq!(out.status.code(), Some(1));
    let d = diags(&out);
    assert_eq!(d.len(), 2, "one diagnostic per item: {d:?}");
    assert_ne!(d[0], d[1], "the two lines must differ: {d:?}");

    // Both items' columns are computed from the probe line, not pinned.
    let first = line.find("Big").expect("first item") + 1;
    let second = line.rfind("Big").expect("second item") + 1;
    assert_ne!(first, second, "the probe must place the items apart");
    assert!(d[0].contains(&format!("(5):{first}:")), "first item at col {first}: {d:?}");
    assert!(d[1].contains(&format!("(5):{second}:")), "second item at col {second}: {d:?}");
}

/// The same, one width up, so what is established is the shared code path
/// rather than `dc.b`'s own arm. `dc.w`, `dc.l` and `dw` take the identical
/// treatment.
#[test]
fn the_per_item_column_is_not_specific_to_dc_b() {
    let line = "\tdc.w\tBig, Big";
    let (out, _) = run(&format!("Big:\tequ\t$123456789\n{line}\n\trts\n"));
    let d = diags(&out);
    assert_eq!(d.len(), 2, "one diagnostic per item: {d:?}");
    assert_ne!(d[0], d[1], "the two lines must differ: {d:?}");
}

/// **The ruling's own gate, in both directions.** The AS surface keeps the
/// parenthesised dialect and the `.emp` surface keeps the colon dialect. This
/// fails if either is rendered in the other's shape, which is the "fixed" into
/// one dialect outcome docs/OVERSEER.md is written to prevent.
#[test]
fn the_two_diagnostic_dialects_stay_apart() {
    let dir = tempfile::tempdir().expect("tempdir");

    let asm = dir.path().join("probe.asm");
    std::fs::write(&asm, "\tcpu 68000\n\torg 0\n\tdc.b nothing_defines_this\n\tend\n")
        .expect("write asm");
    let as_out = Command::new(SIGIL).arg(&asm).output().expect("spawn sigil");
    let as_diag = diags(&as_out);
    assert_eq!(as_diag.len(), 1, "one AS diagnostic expected: {as_diag:?}");
    let as_name = asm.to_string_lossy().into_owned();
    assert!(
        as_diag[0].starts_with(&format!("{as_name}(3):")),
        "the AS surface renders file(line):col: {:?}",
        as_diag[0]
    );
    assert!(
        !as_diag[0].starts_with(&format!("{as_name}:3:")),
        "the AS surface must NOT adopt the .emp dialect: {:?}",
        as_diag[0]
    );

    let emp = dir.path().join("probe.emp");
    std::fs::write(&emp, "module probe\n\nsection code (cpu: m68000, vma: $0) {\n    nop\n}\n")
        .expect("write emp");
    let emp_out = Command::new(SIGIL).arg("emp").arg(&emp).output().expect("spawn sigil");
    let emp_err = String::from_utf8_lossy(&emp_out.stderr).into_owned();
    let emp_name = emp.to_string_lossy().into_owned();
    assert!(
        emp_err.contains(&format!("{emp_name}:4:")),
        "the .emp surface renders path:line:col: {emp_err:?}"
    );
    assert!(
        !emp_err.contains(&format!("{emp_name}(4):")),
        "the .emp surface must NOT adopt the AS dialect: {emp_err:?}"
    );
}
