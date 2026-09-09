//! Asking `sigil` for help gets help, at exit 0, on stdout, and the help a
//! newcomer is shown names every entry point the binary answers to.
//!
//! # What this gates and what it deliberately does not
//!
//! Every expectation here is DERIVED from the `ENTRIES` table in the binary's own
//! source, never transcribed from one measurement of the output. A golden copy of
//! the help text would fail on every wording change and pass on a missing command,
//! which is the drift defect wearing a gate's clothing. So what is asserted is the
//! relationship: the rows in the table are the commands in the list, the commands
//! in the list answer their own `--help`, and asking for help is not an error while
//! getting the arguments wrong still is.
//!
//! The wording of any line, the order of the list and the column layout are all
//! free to change without touching this file.
//!
//! The table is read out of the source text rather than imported because a binary
//! crate's internals are not visible to an integration test. That is also why the
//! extractor carries its own controls: a parse that silently returned nothing would
//! make every assertion below vacuous.

use std::process::Command;

/// The binary under test, built by cargo for this integration target.
const SIGIL: &str = env!("CARGO_BIN_EXE_sigil");

/// The binary's own source, read at compile time.
const SOURCE: &str = include_str!("../src/main.rs");

/// stdout, stderr and exit code of one `sigil` invocation.
fn run(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(SIGIL).args(args).output().expect("run sigil");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().expect("sigil exited with a code"),
    )
}

/// One row of the entry table, as the source declares it.
struct Row {
    /// How the top-level list names the entry point.
    label: String,
    /// The first-argument words that select it. Empty for the bare-file form.
    words: Vec<String>,
}

/// The text of the `ENTRIES` table in the binary's source.
fn entries_table() -> &'static str {
    let start = SOURCE
        .find("const ENTRIES: &[Entry] = &[")
        .expect("main.rs declares the ENTRIES table");
    let rest = &SOURCE[start..];
    let end = rest.find("\n];").expect("the ENTRIES table closes at column zero") + 3;
    &rest[..end]
}

/// The string literal following `key` in `chunk`, if there is one.
fn field(chunk: &str, key: &str) -> Option<String> {
    let at = chunk.find(key)? + key.len();
    let rest = &chunk[at..];
    let close = rest.find('"')?;
    Some(rest[..close].to_string())
}

/// Every row of the entry table.
///
/// The controls matter more than the parse: a table this returned empty, or
/// returned short, would turn every gate in this file green for the wrong reason.
fn rows() -> Vec<Row> {
    let table = entries_table();
    let declared = table.matches("Entry {").count();
    let rows: Vec<Row> = table
        .split("Entry {")
        .skip(1)
        .map(|chunk| {
            let label = field(chunk, "label: \"").expect("every entry declares a label");
            let words_at = chunk.find("words: &[").expect("every entry declares its words")
                + "words: &[".len();
            let words_end =
                chunk[words_at..].find(']').expect("the words list closes") + words_at;
            let words: Vec<String> = chunk[words_at..words_end]
                .split(',')
                .filter_map(|w| {
                    let w = w.trim().trim_matches('"');
                    (!w.is_empty()).then(|| w.to_string())
                })
                .collect();
            Row { label, words }
        })
        .collect();
    assert_eq!(rows.len(), declared, "the table parser lost rows");
    assert!(rows.len() >= 6, "only {} entry points parsed out of the table", rows.len());
    assert!(
        rows.iter().any(|r| r.words.is_empty()),
        "no bare-file row: the parser is reading the wrong text"
    );
    rows
}

/// The command list in a help page: the first token of each line under
/// `commands:`, up to the blank line that ends the block.
///
/// The block is read rather than the whole page because a label also occurs in
/// the page's prose (`sigil emp --help` is the example the footer gives), and a
/// substring search over the page therefore finds a command the list has
/// dropped. Reading the block asks the question the reader's eye asks.
fn listed_commands(page: &str) -> Vec<&str> {
    page.lines()
        .skip_while(|l| l.trim() != "commands:")
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .collect()
}

/// Every conventional way of asking prints the same help, to stdout, at exit 0,
/// and the command list in it is exactly the set of entry points the binary
/// dispatches: none missing, and none listed that does not exist.
#[test]
fn asking_for_help_lists_every_entry_point_and_exits_zero() {
    let rows = rows();
    let mut pages = Vec::new();
    for form in [&["--help"][..], &["-h"][..], &["help"][..]] {
        let (stdout, stderr, code) = run(form);
        assert_eq!(code, 0, "`sigil {}` exited {code}, stderr: {stderr}", form.join(" "));
        assert!(stderr.is_empty(), "`sigil {}` wrote to stderr: {stderr}", form.join(" "));

        let listed = listed_commands(&stdout);
        assert_eq!(
            listed.len(),
            rows.len(),
            "`sigil {}` lists {} commands against {} entry points:\n{stdout}",
            form.join(" "),
            listed.len(),
            rows.len()
        );
        for row in &rows {
            assert!(
                listed.contains(&row.label.as_str()),
                "`sigil {}` does not list the `{}` entry point:\n{stdout}",
                form.join(" "),
                row.label
            );
        }
        pages.push(stdout);
    }
    assert!(
        pages.windows(2).all(|w| w[0] == w[1]),
        "the three ways of asking for help print different things"
    );
}

/// The commands a help page invokes: the token after `sigil ` on each line.
///
/// A substring search over the page cannot answer this, because the commands are
/// substrings of the arguments they take: `emp` is inside `<input.emp>`, so a
/// page that had lost its command still contains the command's name.
fn invoked_commands(page: &str) -> Vec<&str> {
    page.lines()
        .filter_map(|l| l.find("sigil ").map(|at| &l[at + "sigil ".len()..]))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect()
}

/// Every command in the list answers its own `--help`, both as a flag after the
/// command and as `sigil help <command>`, and the page it prints is that
/// command's page rather than another's.
#[test]
fn every_command_answers_its_own_help() {
    let rows = rows();
    let mut checked = 0;
    for row in &rows {
        for word in &row.words {
            for form in [vec![word.as_str(), "--help"], vec!["help", word.as_str()]] {
                let (stdout, stderr, code) = run(&form);
                assert_eq!(code, 0, "`sigil {}` exited {code}, stderr: {stderr}", form.join(" "));
                assert!(
                    invoked_commands(&stdout).contains(&word.as_str()),
                    "`sigil {}` prints a page for some other command:\n{stdout}",
                    form.join(" ")
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 8, "only {checked} per-command help pages were exercised");
}

/// A bare invocation is still a usage error, and it points at help rather than
/// trying to be help.
#[test]
fn a_bare_invocation_is_a_usage_error_that_points_at_help() {
    let (stdout, stderr, code) = run(&[]);
    assert_eq!(code, 2, "a bare `sigil` exited {code}");
    assert!(stdout.is_empty(), "a usage error wrote to stdout: {stdout}");
    assert!(stderr.contains("--help"), "a bare `sigil` does not point at help:\n{stderr}");
}

/// Asking for help is not an error, but getting it wrong still is. Each shape
/// below keeps the exit code it had before help existed.
#[test]
fn getting_it_wrong_is_still_an_error() {
    // A command reached without the arguments it needs: usage error, on stderr.
    for command in ["emp", "test", "parse"] {
        let (stdout, stderr, code) = run(&[command]);
        assert_eq!(code, 2, "`sigil {command}` exited {code}, stderr: {stderr}");
        assert!(stdout.is_empty(), "`sigil {command}` wrote a usage error to stdout: {stdout}");
        assert!(
            stderr.contains("usage: sigil"),
            "`sigil {command}` prints no usage line:\n{stderr}"
        );
    }
    // `build` without its required directory: an error line, then usage.
    let (_, stderr, code) = run(&["build"]);
    assert_eq!(code, 2, "`sigil build` exited {code}");
    assert!(stderr.contains("--aeon"), "`sigil build` does not name what is missing:\n{stderr}");

    // A named command that does not exist is a mistake, not a request.
    let (stdout, stderr, code) = run(&["help", "nosuchcommand"]);
    assert_eq!(code, 2, "`sigil help nosuchcommand` exited {code}");
    assert!(stdout.is_empty(), "an unknown command wrote to stdout: {stdout}");
    assert!(stderr.contains("nosuchcommand"), "the unknown command is not named:\n{stderr}");

    // An input file that is not there: unchanged, and not a usage error.
    let (_, stderr, code) = run(&["no-such-file-9d2f.asm"]);
    assert_eq!(code, 1, "a missing input file exited {code}, stderr: {stderr}");
    assert!(stderr.contains("no-such-file-9d2f.asm"), "the missing file is not named:\n{stderr}");
}

/// `--version` still reports the revision rather than being swallowed by the
/// help path, and it is reachable under both of its spellings.
#[test]
fn version_still_reports_the_revision() {
    for form in ["--version", "-V"] {
        let (stdout, stderr, code) = run(&[form]);
        assert_eq!(code, 0, "`sigil {form}` exited {code}, stderr: {stderr}");
        assert!(stdout.contains("revision"), "`sigil {form}` reports no revision:\n{stdout}");
    }
}
