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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
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

/// The same, run from `dir`. The unknown-command rule asks whether a file of
/// that name exists, so a test of it has to control the directory it asks in:
/// run from the crate root, a probe word would be answered differently the day
/// someone adds a file with that name.
fn run_in(dir: &Path, args: &[&str]) -> (String, String, i32) {
    let out = Command::new(SIGIL).args(args).current_dir(dir).output().expect("run sigil");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().expect("sigil exited with a code"),
    )
}

/// An empty directory of our own under cargo's per-target temp dir, so the
/// probes below run somewhere whose contents this file decides.
fn empty_scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("make the scratch directory");
    assert!(
        std::fs::read_dir(&dir).expect("read the scratch directory").next().is_none(),
        "the scratch directory {} is not empty, so a probe may name a real file",
        dir.display()
    );
    dir
}

/// One row of the entry table, as the source declares it.
struct Row {
    /// How the top-level list names the entry point.
    label: String,
    /// The first-argument words that select it. Empty for the bare-file form.
    words: Vec<String>,
    /// Every option spelling the row lists, dashes and all.
    options: Vec<String>,
}

impl Row {
    /// How a user types this entry point: its first selecting word, or the
    /// label for the bare-file row, which has none. This is the same choice
    /// `suggestion_for` makes, so the two agree without either copying a name.
    fn command(&self) -> &str {
        self.words.first().map(String::as_str).unwrap_or(&self.label)
    }
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
            // The option list: every quoted spelling between `options: &[` and
            // the `]` that closes it. The constructors around them (`flag`,
            // `valued`, `attached`) hold no other string literal, so the quoted
            // runs in that span are exactly the option names.
            let opts_at =
                chunk.find("options: &[").expect("every entry declares its options")
                    + "options: &[".len();
            let opts_end = chunk[opts_at..].find(']').expect("the options list closes") + opts_at;
            let options: Vec<String> = chunk[opts_at..opts_end]
                .split('"')
                .skip(1)
                .step_by(2)
                .map(|o| o.to_string())
                .collect();
            Row { label, words, options }
        })
        .collect();
    assert_eq!(rows.len(), declared, "the table parser lost rows");
    assert!(rows.len() >= 6, "only {} entry points parsed out of the table", rows.len());
    assert!(
        rows.iter().any(|r| r.words.is_empty()),
        "no bare-file row: the parser is reading the wrong text"
    );
    // Controls on the option parse. Without these a parser that returned no
    // options would make every option-derived gate below pass over nothing.
    let total: usize = rows.iter().map(|r| r.options.len()).sum();
    assert!(total >= 15, "only {total} options parsed out of the whole table");
    assert!(
        rows.iter().flat_map(|r| &r.options).all(|o| o.starts_with('-')),
        "an option parsed out of the table does not start with a dash, \
         so the parser is reading the wrong text"
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

/// Every word that a user is plausibly going to type as a command and that is
/// not one, derived from the table: each row's options with their dashes taken
/// off (`check`, which is `build --check`, is the reported case), and a
/// one-character mutation of each command word (`biuld`).
///
/// Words that would defeat the rule under test are dropped rather than
/// asserted about: anything that is a command word after all, and anything
/// holding a `.`, a separator or a leading `-`, which are the signals that say
/// "path" and are gated separately below.
fn probe_words(rows: &[Row]) -> Vec<String> {
    let commands: BTreeSet<&str> =
        rows.iter().flat_map(|r| r.words.iter().map(String::as_str)).collect();
    let mut probes: BTreeSet<String> = BTreeSet::new();
    for row in rows {
        for opt in &row.options {
            probes.insert(opt.trim_start_matches('-').to_string());
        }
        for word in &row.words {
            // One deleted character: a typo of a real command word.
            let mut typo = word.trim_start_matches('-').to_string();
            typo.pop();
            probes.insert(typo);
        }
    }
    probes
        .into_iter()
        .filter(|p| {
            !p.is_empty()
                && !commands.contains(p.as_str())
                && !p.contains('.')
                && !p.contains('/')
                && !p.starts_with('-')
        })
        .collect()
}

/// A word that is not a command must be answered as a command that does not
/// exist, not silently assembled as a filename.
///
/// This is the parcel's defect. `check` is not a command, it is `build
/// --check`; typing `sigil check <args>` used to fall through to the bare-file
/// row, which took the word `check` as its input path and reported `cannot read
/// check` and an assembly failure. `sigil help check` had always said `unknown
/// command`, so the same word got two different answers. Both forms are run
/// here against the same probe and held to the same answer.
///
/// The probe set is derived from the table (see `probe_words`), so an option or
/// a command added tomorrow is probed today.
#[test]
fn a_word_that_is_not_a_command_is_not_taken_as_a_filename() {
    let rows = rows();
    let probes = probe_words(&rows);
    assert!(probes.len() >= 8, "only {} probe words derived from the table", probes.len());
    let dir = empty_scratch("unknown-command");

    for probe in &probes {
        // Control: the rule under test asks whether a file of this name
        // exists. If one did, the probe would be measuring the other branch.
        assert!(
            !dir.join(probe).exists(),
            "the probe word `{probe}` names a real file in {}, so it measures nothing",
            dir.display()
        );

        let (stdout, stderr, code) = run_in(&dir, &[probe]);
        assert_eq!(code, 2, "`sigil {probe}` exited {code}, stderr: {stderr}");
        assert!(stdout.is_empty(), "`sigil {probe}` wrote to stdout: {stdout}");
        assert!(
            stderr.contains(&format!("unknown command '{probe}'")),
            "`sigil {probe}` does not say the command is unknown:\n{stderr}"
        );
        // The defect itself: the word reached the assembler as a path.
        assert!(
            !stderr.contains("cannot read"),
            "`sigil {probe}` still read the command word as a filename:\n{stderr}"
        );
        assert!(
            !stdout.contains("assembly failed"),
            "`sigil {probe}` still tried to assemble the command word:\n{stdout}"
        );
        // It is answered with the whole command list, as `help <word>` is.
        let listed = listed_commands(&stderr);
        assert_eq!(
            listed.len(),
            rows.len(),
            "`sigil {probe}` lists {} commands against {} entry points:\n{stderr}",
            listed.len(),
            rows.len()
        );

        // The asymmetry this parcel closes: both ways of naming the word agree
        // on the verdict and on the exit code.
        let (help_stdout, help_stderr, help_code) = run_in(&dir, &["help", probe]);
        assert_eq!(help_code, code, "`sigil help {probe}` exited {help_code}, `sigil {probe}` {code}");
        assert!(help_stdout.is_empty(), "`sigil help {probe}` wrote to stdout: {help_stdout}");
        assert!(
            help_stderr.contains(&format!("unknown command '{probe}'")),
            "`sigil help {probe}` does not say the command is unknown:\n{help_stderr}"
        );
    }
}

/// A word that some row accepts as an option is answered with the row that
/// takes it and the spelling to type, because that is the mistake actually
/// made: `check` is `build --check`.
///
/// Both halves of the expectation are read out of the table. The row expected
/// is the FIRST one listing the option, which is the rule `suggestion_for`
/// applies, so `-o` (listed by more than one row) has one expected answer here
/// rather than two contradictory ones.
#[test]
fn an_option_typed_as_a_command_names_the_command_that_takes_it() {
    let rows = rows();
    let commands: BTreeSet<&str> =
        rows.iter().flat_map(|r| r.words.iter().map(String::as_str)).collect();
    let mut first_row: BTreeMap<String, (&Row, &String)> = BTreeMap::new();
    for row in &rows {
        for opt in &row.options {
            first_row.entry(opt.trim_start_matches('-').to_string()).or_insert((row, opt));
        }
    }
    let dir = empty_scratch("unknown-command-options");

    let mut checked = 0;
    for (word, (row, opt)) in &first_row {
        if word.is_empty() || commands.contains(word.as_str()) || word.contains('.') {
            continue;
        }
        assert!(!dir.join(word).exists(), "the probe word `{word}` names a real file");
        let (_, stderr, code) = run_in(&dir, &[word.as_str()]);
        assert_eq!(code, 2, "`sigil {word}` exited {code}, stderr: {stderr}");
        let note = stderr
            .lines()
            .find(|l| l.starts_with("note: ") && l.contains(opt.as_str()))
            .unwrap_or_else(|| panic!("`sigil {word}` suggests no option `{opt}`:\n{stderr}"));
        assert!(
            note.contains(row.command()),
            "`sigil {word}` names `{opt}` without naming `{}`, the command that takes it:\n{note}",
            row.command()
        );
        checked += 1;
    }
    assert!(checked >= 15, "only {checked} option words were exercised");
}

/// The other direction, which is what the rule costs: an argument that looks
/// like a path is still handed to the bare-file row, whether or not it is
/// there. Being too eager to say "unknown command" would break `sigil somefile`
/// for a real file whose name carries no extension, so that case is the first
/// one asserted.
#[test]
fn an_argument_that_looks_like_a_path_is_still_a_path() {
    let rows = rows();
    let commands: BTreeSet<&str> =
        rows.iter().flat_map(|r| r.words.iter().map(String::as_str)).collect();
    let dir = empty_scratch("unknown-command-paths");

    // A file that exists and whose name carries no extension: assembled, as
    // before. This is the case the rule must not take from anyone.
    let bare_name = "sourcefile";
    assert!(!commands.contains(bare_name), "`{bare_name}` became a command word; pick another");
    std::fs::write(dir.join(bare_name), "\tcpu\t68000\n\tdc.b\t1\n").expect("write the source");
    let (stdout, stderr, code) = run_in(&dir, &[bare_name, "--hex"]);
    assert_eq!(code, 0, "`sigil {bare_name}` exited {code}, stderr: {stderr}");
    assert!(stdout.contains("01"), "`sigil {bare_name}` did not assemble it:\n{stdout}");

    // A name that is not there but carries the signals of a path is a missing
    // file, at the missing-file exit code, not a usage error about a command.
    for missing in ["nosuch-7c41.asm", "nosuch-7c41/thing", "./nosuch-7c41"] {
        let (_, stderr, code) = run_in(&dir, &[missing]);
        assert_eq!(code, 1, "`sigil {missing}` exited {code}, stderr: {stderr}");
        assert!(
            stderr.contains("cannot read"),
            "`sigil {missing}` is not reported as a file that is not there:\n{stderr}"
        );
        assert!(
            !stderr.contains("unknown command"),
            "`sigil {missing}` was read as a command word:\n{stderr}"
        );
    }
}
