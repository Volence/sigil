//! sigil-cli: the `sigil` command-line assembler binary.
//!
//! **The command line is [`ENTRIES`].** That table is what [`main`] dispatches on
//! and what `--help` prints, so an entry point cannot be reachable without a help
//! line and a help line cannot name an entry point that does not run. A second
//! list written into this note would be a copy nothing executes, so this note
//! names the table rather than restating it.
//!
//! `sigil build --aeon <dir>` is THE Aeon ROM build: it assembles the game's
//! residual root `.asm` (`games/<game>/game_root.asm`) with every `.emp` module
//! lowered natively, chained-links, folds the checksum, emits the sigil-canonical
//! `.lst`, and appends the `convsym` deb2 symbol table: the full shipped ROM.
//!
//! `sigil --version` reports the source revision this executable was built from,
//! which is the only way to tell a current assembler from a stale one: byte
//! identity cannot, because both emit the same ROM when the source is unchanged.

use std::process;

/// The working-tree classifier `build.rs` reaches by path. It is declared here
/// under `cfg(test)` so its unit tests run in the normal suite — a build
/// script's code is otherwise never compiled into any test binary — without
/// linking a module the shipped executable does not call.
#[cfg(test)]
mod tree_class;

/// One entry point of the command line: the words that select it, how help
/// names it, what it does, its full usage text, and the function that runs it.
///
/// [`main`] dispatches by walking [`ENTRIES`] and the help text is rendered from
/// the same rows, so the command list a reader is shown is the command list the
/// binary answers to. The unit tests below hold the rest of that relationship:
/// every row appears in the top-level list, every row's usage names the row, and
/// every flag a row's parser accepts appears in that row's usage.
struct Entry {
    /// First-argument words that select this entry point. Empty for the
    /// bare-file form, which runs whenever the first argument is not one of the
    /// other rows' words.
    words: &'static [&'static str],
    /// How the top-level command list names this entry point.
    label: &'static str,
    /// What this entry point does, in one line, for the top-level list.
    summary: &'static str,
    /// The usage text, one element per printed line.
    usage: &'static [&'static str],
    /// Name of the function whose argument loop accepts this entry point's
    /// flags. The `usage_names_every_accepted_flag` gate reads that function's
    /// match arms out of this source file and holds each flag it accepts to
    /// appearing in `usage`. Only that gate reads it, and it compiles under
    /// `cfg(test)`.
    #[allow(dead_code)]
    flags_fn: &'static str,
    /// The entry point itself. It receives its own row, so the usage it prints
    /// on a missing argument is the row's text rather than a second copy.
    run: fn(&Entry, &[String]),
}

/// Every entry point of the `sigil` command line, in the order help lists them.
const ENTRIES: &[Entry] = &[
    Entry {
        words: &[],
        label: "<input.asm>",
        summary: "assemble one AS-syntax source file to a binary image",
        usage: &["usage: sigil <input.asm> [-o <output.bin>] [--hex]"],
        flags_fn: "run_asm",
        run: run_asm,
    },
    Entry {
        words: &["emp"],
        label: "emp",
        summary: "compile a .emp module or program to a binary image",
        usage: &[
            "usage: sigil emp <input.emp> [--root <dir>] [--prelude <module.id>]",
            "                 [--map <map.toml>] [-o <output.bin>] [--hex] [--deny-todo]",
            "                 [-D NAME=INT]...",
            "note:  --root compiles the whole reachable program under that directory;",
            "       --prelude and --map need it, and are refused by name without it.",
            "note:  --map reads a region map and places each section into its named",
            "       region, against that region's budget. Without it the sections are",
            "       packed sequentially from address 0.",
            "note:  --deny-todo turns every remaining todo hole into an error, so a",
            "       release build cannot ship one.",
        ],
        flags_fn: "run_emp",
        run: run_emp,
    },
    Entry {
        words: &["test"],
        label: "test",
        summary: "run the test blocks in a .emp module, or in every module under a root",
        usage: &[
            "usage: sigil test <input.emp> [-D NAME=INT]...",
            "       sigil test --root <dir> [-D NAME=INT]...",
            "note:  pass EITHER a file OR --root, not both.",
        ],
        flags_fn: "run_test",
        run: run_test,
    },
    Entry {
        words: &["parse"],
        label: "parse",
        summary: "parse one .emp file and report its diagnostics, emitting nothing",
        usage: &["usage: sigil parse <input.emp>"],
        flags_fn: "run_parse",
        run: run_parse,
    },
    Entry {
        words: &["build"],
        label: "build",
        summary: "build the Aeon ROM from a game tree",
        usage: &[
            "usage: sigil build --aeon <dir> [-o <out.bin>] [--emit-lst <lst>]",
            "                   [--game sonic4|demo] [--debug] [--config-a|--config-b|--lean]",
            "                   [--report ram|contracts|indirect-cost]",
            "                   [--extra-entry <module|path.emp>]... [--check]",
            "note:  --extra-entry evaluates the NAMED module's comptime guards; the",
            "       named module must emit nothing (its own imports are not checked)",
            "note:  --check decides every ensure and LinkAssert against final",
            "       post-relaxation placement and writes no ROM; a green check proves nothing",
            "       about region budget or overlap, image bounds, the checksum or the closure",
            "       gate, and is not a statement that the game builds",
            "dev:   --native is an accepted no-op (a native build is the only build).",
            "       --stress-evict and --stress-art each fix an off-canonical development",
            "       shape and take no other shape selector.",
            "env:   SIGIL_WARNINGS=off|summary|full  (warn-tier detail; default summary)",
        ],
        flags_fn: "parse_build_args",
        run: run_build,
    },
    Entry {
        words: &["--version", "-V"],
        label: "--version",
        summary: "print the source revision this assembler was built from",
        usage: &["usage: sigil --version", "       sigil -V"],
        flags_fn: "run_version",
        run: run_version,
    },
];

/// The bare-file row: the one with no selecting word, which runs when the first
/// argument names a file rather than a command.
fn bare_entry() -> &'static Entry {
    ENTRIES
        .iter()
        .find(|e| e.words.is_empty())
        .expect("ENTRIES holds the bare-file row")
}

/// The row `word` selects, if any.
fn entry_for(word: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|e| e.words.contains(&word))
}

/// Every conventional way of asking for help. Asking is not an error: each of
/// these prints help to stdout and exits 0.
fn is_help_word(word: &str) -> bool {
    matches!(word, "--help" | "-h" | "help")
}

/// The top-level help: what the tool is, and one line per row of [`ENTRIES`].
///
/// Rendered from the table rather than written out, so a row added tomorrow is
/// listed here today.
fn top_level_help() -> String {
    let width = ENTRIES.iter().map(|e| e.label.len()).max().unwrap_or(0);
    let mut out = String::new();
    out.push_str("sigil assembles 68000 and Z80 sources, in AS syntax or in the .emp language.\n");
    out.push('\n');
    out.push_str("usage: sigil [<command>] <arguments>\n");
    out.push('\n');
    out.push_str("commands:\n");
    for e in ENTRIES {
        out.push_str(&format!("  {:width$}  {}\n", e.label, e.summary, width = width));
    }
    out.push('\n');
    out.push_str("For one command's arguments, run `sigil <command> --help`, for example\n");
    out.push_str("`sigil emp --help`. `sigil help <command>` prints the same thing.\n");
    out
}

/// One row's own help: what it does, then its usage text.
fn entry_help(entry: &Entry) -> String {
    let mut out = String::new();
    out.push_str(entry.summary);
    out.push_str(".\n\n");
    for line in entry.usage {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    out.push_str("Run `sigil --help` for the other commands.\n");
    out
}

/// Print `entry`'s usage to stderr and exit 2. The caller reached an entry point
/// without the arguments it needs, which is a usage error and not a request for
/// help, so it goes to stderr and keeps its exit code.
fn usage_error(entry: &Entry) -> ! {
    for line in entry.usage {
        eprintln!("{line}");
    }
    eprintln!("run `sigil --help` for the list of commands");
    process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let first = args.get(1).map(String::as_str);

    // `sigil --help`, `sigil -h`, `sigil help`, and `sigil help <command>`.
    if first.is_some_and(is_help_word) {
        match args.get(2) {
            None => print!("{}", top_level_help()),
            Some(word) => match entry_for(word) {
                Some(entry) => print!("{}", entry_help(entry)),
                // Naming a command that does not exist is a mistake, not a
                // request, so it keeps the usage-error exit code.
                None => {
                    eprintln!("error: unknown command '{word}'");
                    eprint!("{}", top_level_help());
                    process::exit(2);
                }
            },
        }
        return;
    }

    // A word selects its row and consumes itself; anything else is the first
    // argument of the bare-file row.
    let (entry, rest) = match first.and_then(entry_for) {
        Some(entry) => (entry, &args[2..]),
        None => (bare_entry(), &args[1..]),
    };

    // `sigil <command> --help` prints that command's usage instead of running it.
    if rest.iter().any(|a| a == "--help" || a == "-h") {
        print!("{}", entry_help(entry));
        return;
    }

    (entry.run)(entry, rest);
}

/// `sigil <input.asm> [-o <output.bin>] [--hex]`: assemble one AS-syntax source
/// file, write or print the image, and end stdout on a line saying how the run
/// ended: [`emit_image`]'s `built:` line on success, [`fail_asm`]'s on failure.
fn run_asm(entry: &Entry, args: &[String]) {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut hex = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                match args.get(i) {
                    Some(path) => output = Some(path.clone()),
                    None => {
                        eprintln!("error: -o requires a path argument");
                        process::exit(2);
                    }
                }
            }
            "--hex" => hex = true,
            other => {
                if input.is_none() {
                    input = Some(other.to_string());
                } else {
                    eprintln!("error: unexpected argument '{other}'");
                    process::exit(2);
                }
            }
        }
        i += 1;
    }

    let input = match input {
        Some(path) => path,
        None => usage_error(entry),
    };

    // `assemble_root` rather than `assemble`: it sets `include_root` to the source file's
    // own parent, which is where an `include` is written relative to. `assemble` leaves it
    // unset and the paths then resolve against whatever directory the command was run
    // from — so `sigil path/to/root.asm` failed for everyone who did not first `cd` into
    // the project, with `cannot include` lines naming files that are plainly there.
    // `assemble_root_located` rather than `assemble_root`: it hands back the
    // `SourceMap` of every spliced file, which is the only thing that can turn a
    // diagnostic's byte span back into `file(line)`. Without it a span is an offset
    // into an unnamed source and the report says only what went wrong, never where
    // — against a 91k-line multi-file program that is not a usable answer, and it
    // is behind what AS itself reports (`smps-bug.asm(9): error: …`).
    // `assemble_root_located_warned` rather than `assemble_root_located`: a
    // `warning` directive is a message the SOURCE AUTHOR wrote to be read, and
    // the run that raises one still SUCCEEDS. The module-only entry point
    // returns `Ok(Module)` and drops it, so the author's line would never reach
    // anyone — the failure mode is silent, which is the one the directive
    // exists to prevent.
    let opts = sigil_frontend_as::Options::default();
    // The `SourceMap` is kept past the front end rather than dropped with the
    // `Assembled`: a LINK diagnostic carries a span into the same spliced files,
    // and rendering it without the map printed a bare `error: …` line that named
    // no file and no line, the one thing a user needs first. Same map, same
    // renderer, so a front-end and a link diagnostic about the same source line
    // print the same way.
    let (module, sources) = match sigil_frontend_as::assemble_root_located_warned(
        std::path::Path::new(&input),
        &opts,
    ) {
        Ok(a) => {
            render_as_messages(&a.messages);
            render_as_warnings(&a);
            (a.module, a.sources)
        }
        Err(failure) => {
            render_as_messages(&failure.messages);
            render_as_diags(&failure);
            fail_asm(failure.diags.len(), Stage::Frontend);
        }
    };
    // `resolve_layout` before `link`, which is what every other FINAL-link route
    // in this binary and in `sigil-harness` does, and what this one has to do to
    // ask the same questions of a program as the ROM build does.
    //
    // `link()` alone answers a strictly smaller set. It is written to serve a
    // PARTIAL link as well as a whole one, so its Pass-1b passes over an
    // `equ_sym` that will not fold — that equate's base may simply be in a
    // module this link was not given — and leaves the name undefined, which is
    // silent until something READS it. `Val equ Missing` with nothing reading
    // `Val` therefore assembled clean here while the same two lines in the ROM
    // build were refused by `fold_equ_syms`, so whether sigil accepted a program
    // depended on which route reached it. `resolve_layout` is where the whole
    // program is visible and where that refusal is taken.
    //
    // It is not only the equates: `resolve_layout` is also the width-relaxation
    // fixpoint, so a `JmpJsrSym`/`RelaxAbsSym`/`RelaxLadder` fragment (which
    // `link()`'s Pass-1c can only refuse, having no placement to choose a width
    // from) now resolves on this route as it does on the others.
    let empty = sigil_ir::SymbolTable::new();
    let resolved = match sigil_link::resolve_layout(&module.sections, &empty, true) {
        Ok(secs) => secs,
        Err(diags) => {
            render_located_diags(&diags, &sources);
            fail_asm(diags.len(), Stage::Layout);
        }
    };
    let linked = match sigil_link::link(&resolved, &empty) {
        Ok(img) => img,
        Err(diags) => {
            render_located_diags(&diags, &sources);
            fail_asm(diags.len(), Stage::Link);
        }
    };
    // The cartridge-window check before `flatten`, located against `resolved`
    // so a section placed outside the window is refused at the line that
    // emitted its byte: `file(line): error: section ...`. `flatten` sizes its
    // buffer from the window, so a refusal here is what stands between an
    // `org -1` and a 4 GiB allocation.
    let bounds = sigil_link::check_image_bounds(&linked, &resolved);
    if !bounds.is_empty() {
        render_located_diags(&bounds, &sources);
        fail_asm(bounds.len(), Stage::Image);
    }
    let image = match sigil_link::flatten(&linked, 0x00) {
        Ok(image) => image,
        Err(msg) => {
            eprintln!("error: {msg}");
            fail_asm(1, Stage::Image);
        }
    };

    // The same tail as `sigil emp`, so the two routes report a finished image in
    // one shape. Its write failure is already on stderr; ending the run is left
    // here so it goes through `fail_asm` like every other failure on this route.
    if emit_image(&image, output.as_deref(), hex).is_err() {
        fail_asm(1, Stage::Image);
    }
}

/// Render AS front-end diagnostics as `file(line): error: message` — the shape AS
/// itself reports, so a user moving off AS reads the same thing in the same place.
///
/// The file named is the one the span belongs to, which for a diagnostic raised
/// inside an `include`d file is THAT file, not the includer: each spliced file is
/// registered under its own [`SourceId`](sigil_span::SourceId) and its lines carry
/// it, so a location survives the splice.
///
/// A diagnostic whose span belongs to no registered file — a whole-run failure such
/// as non-convergence, or a root that never opened — prints bare rather than being
/// attributed to line 1 of some file that had nothing to do with it.
fn render_as_diags(failure: &sigil_frontend_as::Failure) {
    for d in &failure.diags {
        match failure.sources.label(d.primary) {
            Some(loc) => eprintln!("{loc}: {}: {}", d.level, d.message),
            None => eprintln!("{}: {}", d.level, d.message),
        }
    }
}

/// [`render_as_diags`] for a diagnostic list that arrives WITHOUT a `Failure`
/// wrapper (the linker's, which returns bare `Vec<Diagnostic>`) while the map
/// its spans resolve against belongs to the front end that produced the module.
///
/// A link diagnostic used to print as a bare `error: …`, because the caller had
/// already dropped the map. The span was there the whole time; nothing but the
/// map was missing, and a refusal that cannot say which line it is about reads
/// as a fact about the program rather than about a line of it.
fn render_located_diags(diags: &[sigil_span::Diagnostic], sources: &sigil_span::SourceMap) {
    for d in diags {
        match sources.label(d.primary) {
            Some(loc) => eprintln!("{loc}: {}: {}", d.level, d.message),
            None => eprintln!("{}: {}", d.level, d.message),
        }
    }
}

/// The same rendering for a SUCCESSFUL run's warn-tier diagnostics — an AS
/// `warning` directive. Same shape as [`render_as_diags`] on purpose: a reader
/// should not have to learn two formats to tell what the assembler is saying,
/// and asl prints its own warnings and errors in one format too.
///
/// The exit status is untouched: a warning does not fail the assembly (asl exits
/// 0 with `1 warning`, probe `w1`), so this prints and returns.
fn render_as_warnings(assembled: &sigil_frontend_as::Assembled) {
    for d in &assembled.warnings {
        match assembled.sources.label(d.primary) {
            Some(loc) => eprintln!("{loc}: {}: {}", d.level, d.message),
            None => eprintln!("{}: {}", d.level, d.message),
        }
    }
}

/// The AS `message` directive's lines, to STDOUT, one per line, unprefixed.
///
/// Stdout and not the diagnostic stream, because that is where asl writes
/// them: `message` is the author's way of printing a computed value for a
/// human (`s1disasm`'s `Uncompressed driver size: 1BC6h bytes.`), and a
/// build script that captures stdout to read it must find the bare line, not
/// a `file(line): note:` rendering of it. Printed on a failing run as well,
/// before the diagnostics, for the same reason: asl prints the line when it is
/// reached and the failure comes later.
fn render_as_messages(messages: &[String]) {
    for m in messages {
        println!("{m}");
    }
}

/// End a failed `sigil <root.asm>` run, after saying on STDOUT that it failed.
///
/// **Every failure exit in [`run_asm`] goes through here**, which is the whole
/// point: the guarantee is a property of one function rather than of a
/// population of `process::exit(1)` calls that a later edit can quietly leave
/// out. `asm_failure_exits_all_go_through_fail_asm` reads `run_asm`'s own body
/// out of the source and fails if a bare exit reappears in it.
///
/// The line exists because `message` writes to stdout and diagnostics write to
/// stderr, so `sigil root.asm > build.log` on a FAILING run left a log holding
/// the author's reassuring line and not one word about the failure. The
/// asymmetry worth naming is against a C compiler: `cc foo.c > log` on a
/// failing build leaves the log EMPTY, which misleads nobody. Here it left
/// `Uncompressed driver size: 1BC6h bytes.` and exit 1, and a log read later
/// reads as a build that worked.
///
/// **The stream is NOT the fix, and moving it was refused on the consumers.**
/// asl writes `message` to stdout (probe `p1b`), and this surface's job is to
/// be the thing it is compatible with, which is the same argument the two
/// diagnostic dialects are ruled on. `scripts/corpus-baseline.sh` then splits
/// the two streams on purpose and treats stderr as the diagnostic POPULATION:
/// it line-counts it into `NERR`, feeds it the class table, diffs it against a
/// stored baseline with `comm`, and keys its "clean assembly" verdict on that
/// count. Both corpora fire `message` (`s1disasm/sound/z80.asm:231`,
/// `s2disasm/s2.macrosetup.asm:84`), so moving the stream would have inflated
/// the count, added unclassified rows, and made every stored baseline
/// incomparable, to fix a log nobody had to keep.
///
/// The count is the diagnostics this route rendered, so it is derived from the
/// list that was printed rather than tallied separately and left to drift.
///
/// `stopped_at` names the stage that failed, so the run can say which stages did
/// NOT run. See [`Stage`] for why that line exists.
fn fail_asm(errors: usize, stopped_at: Stage) -> ! {
    // Before the count rather than after it, so the failure line stays the LAST
    // thing on stdout. That is the F8 property this function was written for
    // (`asm_failure_line.rs` pins it by reading the last line), and a caveat
    // read before the verdict is no worse than one read after it.
    if let Some(rest) = stopped_at.stages_not_run() {
        println!(
            "this error list may be incomplete: sigil stopped at {}, so {rest} did not run",
            stopped_at.name()
        );
    }
    let noun = if errors == 1 { "error" } else { "errors" };
    println!("assembly failed: {errors} {noun} (reported on stderr)");
    process::exit(1);
}

/// The stages [`run_asm`] runs, in order, for the sake of naming the ones a
/// failure skipped.
///
/// **The list a person reads after a failed run is the diagnostics of ONE
/// stage, and nothing in the output used to say so.** Each stage consumes the
/// previous stage's success value, so a stage that fails takes every later
/// stage's diagnostics with it and there is no continuing past it: the front
/// end returns a `Failure` carrying no `Module`, and `resolve_layout` returns
/// diagnostics carrying no sections. Measured on the reconstructed UXa F5
/// probes under `docs/superpowers/notes/2026-09-10-ux-partial-error-list-repro`:
/// seven errors present and five reported across the front end to link
/// boundary, three present and one reported across layout to link. `s1disasm`
/// is the live case and is worse, because a person outside the process cannot
/// even count what it is holding: its front end is clean, it stops at layout on
/// one colliding-pins error, and the only thing that could count stages 3 to 5
/// is a run that gets past the collision.
///
/// This is the second half of owner ruling `d-28`'s third option, "the
/// assembler says plainly when it is holding errors back, so silence is never
/// mistaken for completeness". `d-28-answered` did not read that half into the
/// yes; it left it as implementation strategy under `d-2`, conditional on
/// measurement AFTER the first half showing errors still withheld at a stage
/// boundary. The measurement above is that condition, met.
///
/// The wording promises only what is known. It does not claim a count, because
/// the count is exactly what the skipped stages could have told us and did not.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// `assemble_root_located_warned`.
    Frontend,
    /// `resolve_layout`: placement, the relaxation fixpoint, the final equ fold.
    Layout,
    /// `link`.
    Link,
    /// `check_image_bounds` and everything after it. Collapsed into one variant
    /// deliberately: `flatten` and `install_artifact` can fail, but neither
    /// raises a diagnostic ABOUT THE PROGRAM that an earlier failure could have
    /// hidden, so a run stopping anywhere in here is holding nothing back and
    /// must not say that it is.
    Image,
}

impl Stage {
    /// What to call this stage to a person. Not the function name: `resolve_layout`
    /// and `check_image_bounds` are ours, and the reader's question is which part
    /// of their build stopped.
    fn name(self) -> &'static str {
        match self {
            Stage::Frontend => "the front end",
            Stage::Layout => "layout",
            Stage::Link => "link",
            Stage::Image => "the image checks",
        }
    }

    /// The stages after this one that can still hold a diagnostic about the
    /// program, or `None` when there are none and the list is therefore whole.
    ///
    /// `None` for [`Stage::Image`] is the control on the line: a variant that
    /// always returned `Some` would put the caveat on every failing run, which
    /// satisfies any gate that looks for it and destroys its meaning.
    fn stages_not_run(self) -> Option<&'static str> {
        match self {
            Stage::Frontend => Some("layout, link and the image checks"),
            Stage::Layout => Some("link and the image checks"),
            Stage::Link => Some("the image checks"),
            Stage::Image => None,
        }
    }
}

/// `sigil --version` / `sigil -V` — report the source revision this executable
/// was built from.
///
/// This exists because byte identity cannot answer the question. A stale
/// assembler and a current one produce identical ROMs whenever the source did
/// not change, so a matching CRC says nothing about which binary produced it.
/// Asking the binary is the only direct answer.
///
/// The banner states the confidence of each claim rather than presenting them
/// as equally solid. `revision` is re-captured by cargo whenever git HEAD or
/// the refs move; `tree` is re-captured whenever a file under this binary's
/// closure paths changes, which is what keeps a `clean` word from standing over
/// a binary linked from uncommitted code. `build.rs` names all of those as rerun
/// triggers and its module note carries the reasoning. `tree` is still labelled
/// a snapshot, because the tracking is path-scoped rather than total, and the
/// output names the ways it can under-report rather than letting a reader assume
/// none.
///
/// `revision` alone answers a question nobody asked, because it moves on every
/// commit in the repository — a lane-log line makes an assembler look stale.
/// `closure-paths` is the set of repository paths cargo compiles this binary
/// from, derived from cargo's own dependency graph, and `closure-revision` is
/// the last commit that touched it. Comparing *those* reports only drift that
/// can reach the executable, and the `tree` state word applies the same set to
/// the working tree. `drift-check` is that comparison with its assembly already
/// done: a whole command rather than a recipe, because a recipe carrying a path
/// list gets assembled wrongly — `build.rs`'s `drift_check` records which shell,
/// and what the wrong assembly returns.
///
/// Every field is a word even when nothing could be determined — an empty
/// string reads as "clean" to a human and passes a grep for a SHA.
fn run_version(_entry: &Entry, _args: &[String]) {
    let revision = env!("SIGIL_REVISION");
    let short = env!("SIGIL_REVISION_SHORT");
    let branch = env!("SIGIL_REVISION_BRANCH");
    let date = env!("SIGIL_REVISION_DATE");
    let tree_state = env!("SIGIL_TREE_STATE");
    let tree_detail = env!("SIGIL_TREE_DETAIL");
    let tree_tracked = env!("SIGIL_TREE_TRACKED");
    let source_dir = env!("SIGIL_SOURCE_DIR");
    let tracks = env!("SIGIL_REVISION_TRACKS");
    let closure_packages = env!("SIGIL_CLOSURE_PACKAGES");
    let closure_paths = env!("SIGIL_CLOSURE_PATHS");
    let closure_note = env!("SIGIL_CLOSURE_NOTE");
    let closure_revision = env!("SIGIL_CLOSURE_REVISION");
    let drift_check = env!("SIGIL_DRIFT_CHECK");
    let published = env!("SIGIL_PUBLISHED");
    let drift_check_published = env!("SIGIL_DRIFT_CHECK_PUBLISHED");
    let error = env!("SIGIL_PROVENANCE_ERROR");

    // The first line is the greppable one: `<name> <semver> (<revision tag>)`.
    // The tag names the code this binary is, so it carries `-dirty` exactly
    // when an uncommitted edit reached that code. Dirt outside those sources
    // leaves the tag bare: it does not change what this executable is, and the
    // `tree:` line below reports it in full.
    let tag = match (revision, tree_state) {
        ("unknown", _) => "revision-unknown".to_string(),
        (_, "dirty") => format!("{short}-dirty"),
        (_, "clean" | "clean-sources") => short.to_string(),
        _ => format!("{short}-tree-unknown"),
    };
    println!("sigil {} ({tag})", env!("CARGO_PKG_VERSION"));

    if revision == "unknown" {
        println!("  revision:  unknown, {error}");
        println!("  tree:      unknown, {tree_detail}");
        println!("  source:    unknown");
        println!("  closure:   unknown, {closure_note}");
        println!("  published: unknown, no revision was captured, so nothing can say whether it reached a remote");
        println!(
            "  freshness: this binary carries NO revision, so nothing here can confirm it \
             matches any source tree. Do not treat it as current."
        );
        return;
    }

    println!("  revision:  {revision}");
    println!("  branch:    {branch}");
    println!("  committed: {date}");
    println!("  tree:      {tree_state} at capture, {tree_detail}");
    // Deliberately `tree-tracked`, not `tree tracked`: aeon's gate reads the
    // state word with `sed -n 's/^ *tree: *//p' | head -1`, and a label sharing
    // that prefix would be a second candidate line for it to pick up.
    println!("  tree-tracked: {tree_tracked}");
    println!("  source:    {source_dir}");
    println!("  closure:   {closure_packages} package(s), {closure_note}");
    println!("  closure-revision: {closure_revision}");
    println!("  closure-paths: {closure_paths}");
    println!("  drift-check: {drift_check}");
    println!("  published: {published}");
    println!("  drift-check-published: {drift_check_published}");
    println!("  freshness: revision and tree state are both re-captured (cargo tracks {tracks}).");
    println!(
        "             tree state is still a build-time snapshot: the tracking is path-scoped, so\n\
         \x20            it can only under-report, and only where no mtime under a watched path\n\
         \x20            moves, dirt OUTSIDE the closure (so `clean` may stand where\n\
         \x20            `clean-sources` is true; neither word is a reason to distrust this\n\
         \x20            binary), a closure path that does not exist yet and so cannot be watched\n\
         \x20            (`tree-tracked` names any), an edit landing inside the cargo invocation\n\
         \x20            that captured this, and any change that alters content without moving an\n\
         \x20            mtime. A word beginning `dirty` is therefore trustworthy when it appears.\n\
         \x20            Compare `revision` against `git rev-parse HEAD` to check this binary\n\
         \x20            against a source tree."
    );
    println!(
        "  drift:     `revision` moves on EVERY commit here, including ones no compilation can\n\
         \x20            see, so comparing it alone warns permanently and therefore says nothing.\n\
         \x20            `closure-paths` is what cargo compiles this binary from, walked from\n\
         \x20            cargo's own dependency graph rather than listed by hand, and\n\
         \x20            `closure-revision` is the last commit that touched it. To check this\n\
         \x20            binary against a tree, run the `drift-check` line above, it is a whole\n\
         \x20            command, paths included, and compare what it prints against\n\
         \x20            `closure-revision`. Equal means no commit in that tree can have reached\n\
         \x20            this binary. It is printed whole rather than as a recipe over\n\
         \x20            `closure-paths` because the obvious assembly of such a recipe puts the\n\
         \x20            list in a shell variable, and a shell that does not word-split an\n\
         \x20            unquoted parameter (zsh) passes it as ONE pathspec: it then matches\n\
         \x20            nothing, prints nothing, and exits 0, so a tree that was never looked at\n\
         \x20            reads as a tree with no drift.\n\
         \x20            The classification OVER-reports and never under-reports. A package\n\
         \x20            carrying a build script contributes its whole directory, since a build\n\
         \x20            script may read any file in it; a `#[cfg(test)]` body or a second binary\n\
         \x20            inside a source directory counts although neither reaches this\n\
         \x20            executable; and an underivable closure counts everything. A commit that\n\
         \x20            adds a package to the graph edits a manifest, which is itself in the\n\
         \x20            closure, so growth is reported rather than missed.\n\
         \x20            What this proves is `cannot affect this binary`, never `the output did\n\
         \x20            not change`, only a rebuild and a byte compare supports the second."
    );
    println!(
        "  anchors:   the two drift checks differ only in what they ask ABOUT. `drift-check`\n\
         \x20            anchors at HEAD of the source tree named above; `drift-check-published`\n\
         \x20            anchors at the remote-tracking ref named in `published`. On a machine\n\
         \x20            where a sibling checkout is a peer's live working tree, that HEAD can be\n\
         \x20            ahead of, behind, or divergent from anything another lane can see, so\n\
         \x20            `behind HEAD` is not a fact until something names what it is behind,\n\
         \x20            which is why both lines say which revision they compare against rather\n\
         \x20            than leaving a reader to guess. The tracking ref is a LOCAL cache of the\n\
         \x20            remote: `git fetch` is what moves it, and `git ls-remote` is what would\n\
         \x20            ask the server. Neither line is a warning; both are positions."
    );
}

/// `sigil parse <input.emp>` — run the .emp lexer/parser front end only and
/// report success (module path + item count) or every diagnostic collected,
/// rendered as `path:line:col: message` via `SourceMap::location`.
fn run_parse(entry: &Entry, args: &[String]) {
    let path = match args.first() {
        Some(path) => path.clone(),
        None => usage_error(entry),
    };

    let src = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {path}: {err}");
            process::exit(1);
        }
    };

    let (file, diags) = sigil_frontend_emp::parse_str(&src);
    if diags.is_empty() {
        println!(
            "{path}: OK, module {}, {} items",
            file.module.path.segments.join("."),
            file.items.len()
        );
        return;
    }

    let mut map = sigil_span::SourceMap::new();
    map.add(src);
    for d in &diags {
        let (line, col) = map.location(d.primary);
        println!("{path}:{line}:{col}: {}", d.message);
    }
    process::exit(1);
}

/// Compile a Spec 2 `.emp` source string to its flat linked binary image.
/// Mirrors the top-level `.asm` path but through the emp front end: parse →
/// [`lower_module`](sigil_frontend_emp::lower::lower_module) (threading
/// `include_root` so comptime `embed`/`import` resolve against the source
/// directory, §6.7) → [`resolve_layout`](sigil_link::resolve_layout) (emp defers
/// jmp/jsr width + layout to link, D-P4.2) → [`link`](sigil_link::link) →
/// [`flatten`](sigil_link::flatten). Returns the image bytes (or `None` if a
/// hard error stopped compilation) plus ALL diagnostics collected; the caller
/// renders them and treats any `Error`-level diagnostic as fatal.
fn compile_emp(
    src: &str,
    include_root: Option<&std::path::Path>,
    defines: &[(String, i128)],
) -> (Option<Vec<u8>>, Vec<sigil_span::Diagnostic>) {
    let (file, mut diags) = sigil_frontend_emp::parse_str(src);
    if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return (None, diags);
    }
    let opts = sigil_frontend_emp::lower::LowerOptions {
        initial_cpu: sigil_ir::Cpu::M68000,
        include_root: include_root.map(std::path::Path::to_path_buf),
        embed_base: None,
        defines: defines.to_vec(),
    };
    let (module, lower_diags) = sigil_frontend_emp::lower::lower_module(&file, &opts);
    diags.extend(lower_diags);
    if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return (None, diags);
    }
    match link_sections(&module.sections, &module.link_asserts) {
        Ok((image, mut warns)) => {
            diags.append(&mut warns);
            (Some(image), diags)
        }
        Err(mut ds) => {
            diags.append(&mut ds);
            (None, diags)
        }
    }
}

/// The shared emp link prefix: `resolve_layout` (emp defers jmp/jsr width +
/// layout to link) → `link` → the deferred link-assertion checker (D-H.6), against
/// one flat empty [`SymbolTable`] so cross-module (and cross-section) references
/// resolve. The two link tails — `flatten` (no map) and `emit_rom` (map) — reuse
/// this identical prefix, so they differ only in the final materialization step.
/// Byte-identical whether fed one module's sections or a whole concatenated
/// program. A failing deferred `ensure`/`ensure_fatal` (D-H.4) is an `Error`
/// diagnostic here — folded against the POST-relaxation symbol table (`asserts`
/// empty ⇒ no check, byte-neutral).
fn link_to_image(
    sections: &[sigil_ir::Section],
    asserts: &[sigil_ir::LinkAssert],
) -> Result<
    (sigil_link::LinkedImage, Vec<sigil_ir::Section>, Vec<sigil_span::Diagnostic>),
    Vec<sigil_span::Diagnostic>,
> {
    let empty = sigil_ir::SymbolTable::new();
    let resolved = sigil_link::resolve_layout(sections, &empty, true)?;
    let image = sigil_link::link(&resolved, &empty)?;
    // The link succeeded and labels are at their final post-relaxation VMAs — now
    // decide the deferred guards against exactly those addresses (D-H.6/D-H.7).
    // Warning-tier assert failures ([layout.odd-item] on data, D2.29) ride the
    // Ok path so they surface without failing the build.
    let assert_diags = sigil_link::check_link_asserts(&resolved, &empty, asserts);
    if assert_diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(assert_diags);
    }
    // The resolved sections ride along: they carry the post-placement LMAs and
    // the fragment spans a tail needs to locate a refusal about the image.
    Ok((image, resolved, assert_diags))
}

/// The no-map link seam: [`link_to_image`], the cartridge-window check
/// (located against the resolved sections, so a section outside the window is
/// refused at the line that emitted its byte), then `flatten` (gap-fill 0x00).
/// No game-map region validation happens here; `link_rom` is the map tail.
fn link_sections(
    sections: &[sigil_ir::Section],
    asserts: &[sigil_ir::LinkAssert],
) -> Result<(Vec<u8>, Vec<sigil_span::Diagnostic>), Vec<sigil_span::Diagnostic>> {
    let (image, resolved, warns) = link_to_image(sections, asserts)?;
    let bounds = sigil_link::check_image_bounds(&image, &resolved);
    if !bounds.is_empty() {
        return Err(bounds);
    }
    let bytes = sigil_link::flatten(&image, 0x00).map_err(|msg| vec![unlocated_error(msg)])?;
    Ok((bytes, warns))
}

/// An error diagnostic about the whole image rather than a source line. An id
/// past every scanned file makes the renderers degrade to a bare
/// `error: <msg>` rather than attribute it to whichever module happens to hold
/// `SourceId(0)`.
fn unlocated_error(message: String) -> sigil_span::Diagnostic {
    sigil_span::Diagnostic {
        level: sigil_span::Level::Error,
        message,
        primary: sigil_span::Span { source: sigil_span::SourceId(u32::MAX), start: 0, end: 0 },
    }
}

/// Install one output artifact at `path`, by rename rather than by truncation.
///
/// EVERY file this binary writes for a consumer goes through here, and the reason it
/// is one function rather than a call at each site is that the guarantee is only
/// worth anything if it holds at all of them: a lane polling the ROM while the
/// listing beside it is still written in place learns nothing from the ROM's
/// atomicity. A new output artifact belongs on this path too.
///
/// The mechanism, the limits of the guarantee, and the file-mode contract a rename
/// answers differently from a truncation are in
/// [`sigil_harness::atomic_write`](sigil_harness::atomic_write), which this
/// delegates to unchanged. In particular a failed write leaves the previous
/// artifact complete and readable rather than a stub, which is what makes exiting
/// non-zero on the error below a safe response.
fn install_artifact(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    sigil_harness::atomic_write::write_atomic_io(std::path::Path::new(path), bytes)
}

/// A write to `-o`'s path failed. [`emit_image`] has already said so on stderr by
/// the time a caller sees this; what is left to the caller is how the run ends.
struct WriteFailed;

/// The shared output tail of `sigil <input.asm>` and `sigil emp`: write `image` to
/// `output` (if given), print it as `--hex` (if set), and report the build.
///
/// The success line states the DISPOSITION of the image, not only its size, because
/// those are two different facts and a reader has no other channel for the second
/// one. Building without `-o` is legitimate (`--hex`, a syntax check), so it is not
/// an error, but a run that discarded the image must not be indistinguishable from
/// one that left a ROM on disk: that difference is the whole cost of a stale
/// artifact believed fresh. The wrote-case names the path, so the line answers
/// "where is it" as well as "did it happen".
///
/// `built: N bytes` stays the prefix in both cases. The byte count is the fact both
/// outcomes share, and it is what the acceptance gates assert on; the disposition is
/// the suffix that separates them.
///
/// The line goes to STDOUT and comes LAST, after the `--hex` line. That makes it
/// the success half of what `fail_asm` is on the AS path: whichever way a run
/// ends, the last line of stdout says how, so a captured `> build.log` always
/// ends on the verdict. It is not on stderr because stderr is the diagnostic
/// stream, and `scripts/corpus-baseline.sh` reads a clean assembly off an empty
/// one. A consumer of `--hex` reads the hex line, not the whole of stdout; on the
/// AS path, `message` lines already print ahead of it.
///
/// A failed write is reported here and returned rather than exited on, so each
/// route ends the run its own way: `run_asm` through `fail_asm`, which every one
/// of its failure exits must use.
fn emit_image(image: &[u8], output: Option<&str>, hex: bool) -> Result<(), WriteFailed> {
    if let Some(out_path) = output {
        if let Err(err) = install_artifact(out_path, image) {
            eprintln!("error: cannot write {out_path}: {err}");
            return Err(WriteFailed);
        }
    }
    if hex {
        let rendered: Vec<String> = image.iter().map(|b| format!("{b:02X}")).collect();
        println!("{}", rendered.join(" "));
    }
    match output {
        Some(out_path) => println!("built: {} bytes, wrote {out_path}", image.len()),
        None => println!(
            "built: {} bytes, no file written (pass -o <path> to write one)",
            image.len()
        ),
    }
    Ok(())
}

/// Consume the value following a value-taking flag at `args[*i]`, advancing `i`.
/// A missing value — or one that looks like another flag (`-`-prefixed) — is a
/// usage error (exit 2), so e.g. `--root -o` cannot silently swallow `-o` as the
/// root directory.
fn flag_value(args: &[String], i: &mut usize, flag: &str) -> String {
    *i += 1;
    match args.get(*i) {
        Some(v) if !v.starts_with('-') => v.clone(),
        _ => {
            eprintln!("error: {flag} requires a value argument");
            process::exit(2);
        }
    }
}

/// Parse one `-D NAME=INT` argument (sound-migration T2 Task 1, R1) into a
/// `(name, value)` define pair. `INT` accepts the same int-literal shapes as
/// the rest of the CLI's ROM tooling: plain decimal (optionally `-`-signed),
/// `$hex`, and `0x`hex — a strict superset of the `.emp` lexer's own int forms
/// (which has `$hex` but no `0x`), since a CLI flag is not source text a
/// diagnostic ever points back into. A malformed `NAME=INT` (no `=`, an empty
/// NAME, or a non-integer value) is a usage error (exit 2), reported
/// immediately rather than deferred to a confusing downstream
/// `[defines.collision]`-shaped message.
fn parse_define(arg: &str) -> (String, i128) {
    let Some((name, value)) = arg.split_once('=') else {
        eprintln!("error: -D expects NAME=INT, got '{arg}'");
        process::exit(2);
    };
    if name.is_empty() {
        eprintln!("error: -D expects NAME=INT, got '{arg}' (empty name)");
        process::exit(2);
    }
    let Some(parsed) = parse_define_int(value) else {
        eprintln!("error: -D {name}=... value '{value}' is not an integer (decimal, $hex, or 0x hex)");
        process::exit(2);
    };
    (name.to_string(), parsed)
}

/// Parse a single `-D` int literal: `$hex`, `0x`/`0X` hex, or decimal (with an
/// optional leading `-`). Returns `None` for anything else, including empty
/// input or a hex/decimal run with invalid digits.
fn parse_define_int(s: &str) -> Option<i128> {
    if let Some(digits) = s.strip_prefix('$') {
        return i128::from_str_radix(digits, 16).ok();
    }
    if let Some(digits) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i128::from_str_radix(digits, 16).ok();
    }
    s.parse::<i128>().ok()
}

/// `sigil test <input.emp> [--root <dir>] [-D NAME=INT]...` — run the file's
/// `comptime test` blocks (S2-D11(a)). With `--root`, every module in the
/// manifest is swept (each module's tests run MODULE-LOCAL — the colocated
/// case; cross-module imports in test bodies are the recorded next
/// increment). Output: one `test <module>::<name> ... ok|FAILED` line per
/// test, failure diagnostics indented beneath, then a cargo-style summary.
/// Exit 0 iff every test passed (and no module failed to parse).
fn run_test(entry: &Entry, args: &[String]) {
    let mut input: Option<String> = None;
    let mut root_arg: Option<String> = None;
    let mut defines: Vec<(String, i128)> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--root" => root_arg = Some(flag_value(args, &mut i, "--root")),
            "-D" => defines.push(parse_define(&flag_value(args, &mut i, "-D"))),
            other => {
                if input.is_none() {
                    input = Some(other.to_string());
                } else {
                    eprintln!("error: unexpected argument '{other}'");
                    process::exit(2);
                }
            }
        }
        i += 1;
    }

    // Gather (path, source) pairs: the single file, or every module file
    // under --root (the manifest's own discovery, so `sigil test --root` and
    // `sigil emp --root` agree about what a module is).
    let mut broken_modules = 0usize;
    let files: Vec<(String, String)> = match (&input, &root_arg) {
        (Some(path), None) => match std::fs::read_to_string(path) {
            Ok(src) => vec![(path.clone(), src)],
            Err(err) => {
                eprintln!("error: cannot read {path}: {err}");
                process::exit(1);
            }
        },
        (Some(_), Some(_)) => {
            // Ambiguous: sweep the root, or just the file? Refuse rather
            // than silently pick (Item-10 review m2).
            eprintln!("error: pass EITHER <input.emp> OR --root <dir>, not both");
            process::exit(2);
        }
        (None, Some(root)) => {
            let (manifest, mdiags) =
                sigil_frontend_emp::resolve::manifest::Manifest::scan(std::path::Path::new(root));
            // Only STRUCTURAL scan failures abort the sweep; a module that
            // fails to PARSE is counted broken below and the other modules'
            // tests still run (Item-10 review M2).
            if mdiags.iter().any(|d| d.message.contains("cannot read module root")) {
                render_program_diags(&manifest, &mdiags);
                process::exit(1);
            }
            manifest
                .modules
                .iter()
                .filter_map(|m| {
                    let src = std::fs::read_to_string(&m.path).ok();
                    if src.is_none() {
                        eprintln!("error: cannot read {}", m.path.display());
                        broken_modules += 1;
                    }
                    src.map(|src| (m.path.display().to_string(), src))
                })
                .collect()
        }
        (None, None) => usage_error(entry),
    };

    let root_include = root_arg
        .as_deref()
        .and_then(|r| std::fs::canonicalize(r).ok());
    let mut total = 0usize;
    let mut failed = 0usize;
    for (path, src) in files {
        let (file, pdiags) = sigil_frontend_emp::parse_str(&src);
        if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
            let mut map = sigil_span::SourceMap::new();
            map.add(src.clone());
            for d in &pdiags {
                let (line, col) = map.location(d.primary);
                eprintln!("{path}:{line}:{col}: {}", d.message);
            }
            broken_modules += 1;
            continue;
        }
        let module_id = file.module.path.segments.join(".");
        // `--root` mode: embed/import resolve against the ROOT (matching
        // `sigil emp --root`, Item-10 review m1); single-file mode keeps the
        // file's own directory (matching `sigil emp <file>`).
        let include_root = match &root_include {
            Some(r) => Some(r.clone()),
            None => {
                let parent =
                    std::path::Path::new(&path).parent().unwrap_or(std::path::Path::new(""));
                let root_dir =
                    if parent.as_os_str().is_empty() { std::path::Path::new(".") } else { parent };
                std::fs::canonicalize(root_dir).ok()
            }
        };
        let results =
            sigil_frontend_emp::eval::run_module_tests(&file, include_root.as_deref(), &defines);
        let mut map = sigil_span::SourceMap::new();
        map.add(src.clone());
        for r in results {
            total += 1;
            if r.passed {
                println!("test {module_id}::{} ... ok", r.name);
            } else {
                failed += 1;
                println!("test {module_id}::{} ... FAILED", r.name);
                for d in &r.diags {
                    let (line, col) = map.location(d.primary);
                    println!("    {path}:{line}:{col}: {}", d.message);
                }
            }
        }
    }
    println!(
        "test result: {}. {} passed; {} failed{}",
        if failed == 0 && broken_modules == 0 { "ok" } else { "FAILED" },
        total - failed,
        failed,
        if broken_modules > 0 {
            format!("; {broken_modules} module(s) failed to parse")
        } else {
            String::new()
        }
    );
    if failed > 0 || broken_modules > 0 {
        process::exit(1);
    }
}

/// `--deny-todo` (S2-D11(e)): promote every `[todo.present]` hole to an error
/// so a release build cannot ship one. A post-filter at the CLI layer — the
/// frontend stays flag-free, and `unreachable!` (which never reports) is
/// untouched by construction. `build` gains the flag when the mixed Aeon build
/// first carries a `todo!` (no consumer today).
fn promote_todo_holes(diags: &mut [sigil_span::Diagnostic], deny_todo: bool) {
    if !deny_todo {
        return;
    }
    for d in diags.iter_mut() {
        if d.message.starts_with("[todo.present]") {
            d.level = sigil_span::Level::Error;
        }
    }
}

/// `sigil emp <input.emp> [-o <output.bin>] [--hex]` — compile a Spec 2 `.emp`
/// module to a flat binary image. `embed`/`import` paths resolve against the
/// source file's own directory (the capability-sandbox include-root, §6.7),
/// canonicalized so a comptime capture path is stable regardless of cwd.
fn run_emp(entry: &Entry, args: &[String]) {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut root_arg: Option<String> = None;
    let mut prelude: Option<String> = None;
    let mut map_arg: Option<String> = None;
    let mut hex = false;
    let mut deny_todo = false;
    let mut defines: Vec<(String, i128)> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => output = Some(flag_value(args, &mut i, "-o")),
            "--root" => root_arg = Some(flag_value(args, &mut i, "--root")),
            "--prelude" => prelude = Some(flag_value(args, &mut i, "--prelude")),
            "--map" => map_arg = Some(flag_value(args, &mut i, "--map")),
            "--hex" => hex = true,
            "--deny-todo" => deny_todo = true,
            "-D" => defines.push(parse_define(&flag_value(args, &mut i, "-D"))),
            other => {
                if input.is_none() {
                    input = Some(other.to_string());
                } else {
                    eprintln!("error: unexpected argument '{other}'");
                    process::exit(2);
                }
            }
        }
        i += 1;
    }

    let input = match input {
        Some(path) => path,
        None => usage_error(entry),
    };

    // Multi-module path: `--root <dir>` gathers, resolves, and links the whole
    // reachable program. Single-file path (no `--root`) is unchanged.
    if let Some(root_dir) = root_arg {
        run_emp_program(
            &input,
            &root_dir,
            prelude.as_deref(),
            map_arg.as_deref(),
            output.as_deref(),
            hex,
            deny_todo,
            &defines,
        );
        return;
    }
    // Flags the single-file path cannot honour are refused BY NAME here rather than
    // dropped. A flag that is parsed, consumed, and ignored makes a run that did not
    // do what was asked indistinguishable from one that did, and the flag's own
    // spelling is the only thing that can point at the mistake.
    if map_arg.is_some() {
        eprintln!(
            "error: --map requires --root (region placement is a multi-module concern); \
             pass --root <dir>, or drop --map"
        );
        process::exit(2);
    }
    if prelude.is_some() {
        eprintln!(
            "error: --prelude requires --root (a prelude is a module id resolved under the \
             scan root, not a file path); pass --root <dir>, or drop --prelude"
        );
        process::exit(2);
    }

    let src = match std::fs::read_to_string(&input) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {input}: {err}");
            process::exit(1);
        }
    };

    // Include-root = the source file's own directory (empty parent → cwd),
    // canonicalized so the sandbox and capture ledger see a stable absolute path.
    let parent = std::path::Path::new(&input).parent().unwrap_or(std::path::Path::new(""));
    let root_dir = if parent.as_os_str().is_empty() { std::path::Path::new(".") } else { parent };
    let root = std::fs::canonicalize(root_dir).ok();
    let (image, mut diags) = compile_emp(&src, root.as_deref(), &defines);
    promote_todo_holes(&mut diags, deny_todo);

    if !diags.is_empty() {
        let mut map = sigil_span::SourceMap::new();
        map.add(src);
        for d in &diags {
            let (line, col) = map.location(d.primary);
            eprintln!("{input}:{line}:{col}: {}", d.message);
        }
    }

    let fatal = diags.iter().any(|d| d.level == sigil_span::Level::Error);
    let image = match image {
        Some(img) if !fatal => img,
        _ => process::exit(1),
    };

    if emit_image(&image, output.as_deref(), hex).is_err() {
        process::exit(1);
    }
}

/// The multi-module `sigil emp <entry> --root <dir>` path: scan the root, derive
/// the entry module id from the entry path, build the whole reachable program
/// ([`build_program`](sigil_frontend_emp::resolve::build_program)), and — if no
/// error diagnostics — run the same `resolve_layout` → `link` → `flatten` seam as
/// the single-file path. Diagnostics render as `path:line:col: message` using a
/// [`SourceMap`](sigil_span::SourceMap) rebuilt in the manifest's SourceId order.
#[allow(clippy::too_many_arguments)] // internal driver; mirrors run_emp's flag set
fn run_emp_program(
    input: &str,
    root_dir: &str,
    prelude: Option<&str>,
    map_path: Option<&str>,
    output: Option<&str>,
    hex: bool,
    deny_todo: bool,
    defines: &[(String, i128)],
) {
    use sigil_frontend_emp::resolve;
    use std::path::Path;

    let (manifest, mut diags) = resolve::manifest::Manifest::scan(Path::new(root_dir));

    let entry_id = match resolve::entry_id_for_path(&manifest, Path::new(input)) {
        Some(id) => id,
        None => {
            // Surface the manifest's own diagnostics FIRST: a mistyped/nonexistent
            // `--root` makes `scan` emit `cannot read module root …` AND yields no
            // modules (so entry-id resolution fails) — rendering only the generic
            // "not a module under --root" would bury the real cause.
            render_program_diags(&manifest, &diags);
            if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
                process::exit(1);
            }
            eprintln!("error: entry file {input} is not a module under --root {root_dir}");
            process::exit(1);
        }
    };

    let include_root = std::fs::canonicalize(root_dir).ok();
    let opts = sigil_frontend_emp::lower::LowerOptions {
        initial_cpu: sigil_ir::Cpu::M68000,
        include_root,
        embed_base: None,
        defines: defines.to_vec(),
    };

    // `link_asserts`: deferred link-time guards (D-H.4), decided by the link tails
    // below against the post-relaxation symbol table.
    let (mut sections, link_asserts, mut pdiags) =
        resolve::build_program(&manifest, &entry_id, prelude, &opts);
    diags.append(&mut pdiags);
    promote_todo_holes(&mut diags, deny_todo);

    render_program_diags(&manifest, &diags);
    if diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        process::exit(1);
    }

    // `--map`: load the region map, place each section into its named region, then
    // link and emit through `emit_rom` (which validates each section's region
    // budget, §7.3). Without `--map`, keep today's `flatten` behavior unchanged.
    let image = match map_path {
        Some(path) => {
            let toml = match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(err) => {
                    eprintln!("error: cannot read {path}: {err}");
                    process::exit(1);
                }
            };
            let map = match sigil_link::load_map(&toml) {
                Ok(m) => m,
                Err(err) => {
                    eprintln!("error: cannot load map {path}: {err}");
                    process::exit(1);
                }
            };
            let pdiags = resolve::place_sections(&mut sections, &map);
            render_program_diags(&manifest, &pdiags);
            if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
                process::exit(1);
            }
            match link_rom(&sections, &link_asserts, &map) {
                Ok((rom, warns)) => {
                    render_program_diags(&manifest, &warns);
                    rom
                }
                Err(ds) => {
                    render_program_diags(&manifest, &ds);
                    process::exit(1);
                }
            }
        }
        None => {
            // No `--map`: nothing would otherwise place these sections, so every
            // module's section would keep `lma == 0` and overlap at the origin
            // (BUG I3). Pack them sequentially from 0 so cross-module branches
            // resolve to distinct, non-overlapping addresses (single reachable
            // module → one section at 0, unchanged).
            resolve::place_sequential(&mut sections, 0);
            match link_sections(&sections, &link_asserts) {
                Ok((image, warns)) => {
                    render_program_diags(&manifest, &warns);
                    image
                }
                Err(ds) => {
                    render_program_diags(&manifest, &ds);
                    process::exit(1);
                }
            }
        }
    };

    if emit_image(&image, output, hex).is_err() {
        process::exit(1);
    }
}

/// Region-placed emp link seam: `resolve_layout` → `link` → deferred-assert check
/// (D-H.6) → `emit_rom` against the memory map, so each section is validated for
/// region containment/budget (§7.3) and gaps are filled with the map's default
/// byte. A failing deferred guard (D-H.4) surfaces as a proper span-carrying
/// diagnostic (same channel as the no-map tail); an `emit_rom` region/placement
/// error is wrapped as a single null-span diagnostic.
fn link_rom(
    sections: &[sigil_ir::Section],
    asserts: &[sigil_ir::LinkAssert],
    map: &sigil_ir::map::MemoryMap,
) -> Result<(Vec<u8>, Vec<sigil_span::Diagnostic>), Vec<sigil_span::Diagnostic>> {
    let (linked, _resolved, warns) = link_to_image(sections, asserts)?;
    // A region/placement failure belongs to no source line.
    sigil_link::emit_rom(&linked, map).map(|rom| (rom, warns)).map_err(|msg| vec![unlocated_error(msg)])
}

/// Render multi-module diagnostics as `path:line:col: <level>: message`, through
/// the same [`SourceIndex`](sigil_frontend_emp::resolve::manifest::SourceIndex) the
/// warn tier renders with, so both tiers read as one system. A diagnostic whose
/// source the index cannot locate falls back to `<level>: message`.
fn render_program_diags(
    manifest: &sigil_frontend_emp::resolve::manifest::Manifest,
    diags: &[sigil_span::Diagnostic],
) {
    if diags.is_empty() {
        return;
    }
    let index = sigil_frontend_emp::resolve::manifest::SourceIndex::new(manifest);
    for d in diags {
        match index.locate(d.primary) {
            Some(loc) => eprintln!("{loc}: {}: {}", d.level, d.message),
            None => eprintln!("{}: {}", d.level, d.message),
        }
    }
}

/// `sigil build --aeon <dir> [-o <output.bin>] [--emit-lst <lst>]` — THE Aeon ROM
/// build (post-flip: the ONLY build).
///
/// Drives the SAME code path the native gates bank — assemble (all `.emp` modules
/// lowered + AS residual) → declared-order chained link → `emit_rom` (checksum
/// folded) → sigil-canonical `.lst` → `convsym` deb2 appendix → `fixheader` — and
/// writes the full ROM+appendix. Target selected by `--game <sonic4|demo>` +
/// `--debug` (or `--config-a`/`--config-b`/`--lean` for the off-canonical proof shapes).
/// This is what `build.sh` invokes. The legacy no-appendix all-AS `assemble_full_rom`
/// mode retired with the flip (the AS-reassembly harness is gone); `--native` is
/// accepted as a no-op for build.sh compatibility.
fn run_build(entry: &Entry, args: &[String]) {
    let opts = match parse_build_args(args) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("error: {msg}");
            usage_error(entry);
        }
    };
    let aeon_path = std::path::Path::new(&opts.aeon);

    // A native build emits the sound artifacts INTO this tree
    // (`native::ensure_generated`), and that write's precondition
    // (`seam2::require_named_reference_tree`) asks the tree to have been NAMED rather
    // than resolved from a hardcoded fallback. `--aeon` names it — aeon's `build.sh`
    // runs `sigil build --aeon . --native` from the tree it is building — so the
    // argument is published as this process's `AEON_DIR`. One rule then covers every
    // writing process, and the argv caller is not an exception to it. Set before any
    // build work, while the process is still single-threaded.
    if std::env::var_os("AEON_DIR").is_none() {
        std::env::set_var("AEON_DIR", aeon_path);
    }

    match opts.report {
        Some(ReportKind::Ram) => run_ram_report(aeon_path, &opts.target),
        Some(ReportKind::Contracts) => run_contract_report(aeon_path, &opts.target),
        Some(ReportKind::IndirectCost) => run_indirect_cost_report(aeon_path, &opts.target),
        None if opts.check => run_check_native(aeon_path, &opts),
        None => run_build_native(aeon_path, &opts),
    }
}

/// `--report ram` (T1): print the RAM map for the selected target — one row per
/// `region`, with its base/end address, allocated size, alignment/pad padding, budget
/// limit, and headroom. The numbers come from the SAME region resolver the build runs
/// (`resolve::build_ram_report` over the frontend's region layout), against the
/// target's shipping `-D` define set (so the DEBUG shape's `game_ram` +4 shows).
///
/// The RAM modules are not `use`-reachable (their `pub vars` are cross-seam link
/// labels no module imports), so the region-module set is passed EXPLICITLY: the
/// engine RAM plus the selected game's RAM module (from the native profile). The
/// [`RamRegionRow`](sigil_frontend_emp::lower::RamRegionRow) data shape is deliberately
/// render-free so a future Spec-3 editor inlay-hint surface can reuse it directly.
fn run_ram_report(aeon: &std::path::Path, target: &BuildTarget) {
    use sigil_frontend_emp::resolve;

    // The target's shipping profile supplies the game RAM module + the exact `-D`
    // define set the `.emp` RAM modules read (SYSTEM_STACK, DEBUG, the game sizing
    // consts engine.ram consumes: MAX_RING_BUFFER / COLLECTED_WINDOW_SLOTS / …).
    let (label, profile) = target.label_and_profile();
    let defines = shape_defines_or_exit(&profile, aeon);
    let manifest = scan_or_exit(aeon);

    let opts = sigil_frontend_emp::lower::LowerOptions {
        initial_cpu: sigil_ir::Cpu::M68000,
        include_root: std::fs::canonicalize(aeon).ok(),
        embed_base: None,
        defines: defines.clone(),
    };

    // Engine RAM + the game's RAM module (the two region-owning modules for this game).
    let region_ids: [&str; 2] = ["engine.ram", profile.game_ram_module];
    // Errors always render in full; the warn tier obeys `SIGIL_WARNINGS` here for
    // the same reason it does in the build — one policy, one channel.
    let (rows, diags) = resolve::build_ram_report(&manifest, &region_ids, &opts);
    let errors: Vec<_> =
        diags.iter().filter(|d| d.level == sigil_span::Level::Error).cloned().collect();
    render_program_diags(&manifest, &errors);
    let index = sigil_frontend_emp::resolve::manifest::SourceIndex::new(&manifest);
    report_warnings(&sigil_harness::native::collect_warnings(&index, &[&diags], None));
    if !errors.is_empty() {
        process::exit(1);
    }

    print_report_header("RAM map", &label, &defines);
    print_ram_report(&rows);
}

/// Scan `aeon` for `.emp` modules, or render the reason and exit. Every report
/// starts here: errors render in full and stop the run, and the scan's OWN warn
/// tier — the `[module.path-mismatch]` family, which no later stage re-reports —
/// goes through the one `SIGIL_WARNINGS` channel. A report that swallows the
/// manifest's warnings shows a cleaner tree than the build does.
fn scan_or_exit(aeon: &std::path::Path) -> sigil_frontend_emp::resolve::manifest::Manifest {
    use sigil_frontend_emp::resolve::manifest::{Manifest, SourceIndex};
    let (manifest, mdiags) = Manifest::scan(aeon);
    let errors: Vec<_> =
        mdiags.iter().filter(|d| d.level == sigil_span::Level::Error).cloned().collect();
    render_program_diags(&manifest, &errors);
    let index = SourceIndex::new(&manifest);
    report_warnings(&sigil_harness::native::collect_warnings(&index, &[&mdiags], None));
    if !errors.is_empty() {
        process::exit(1);
    }
    manifest
}

/// The shape's merged comptime define set: the profile's built-in rows + the
/// game's own `map.toml [defines]` rows. A malformed table, a duplicated key, or
/// a game row shadowing a built-in is a config error that stops the run here.
fn shape_defines_or_exit(
    profile: &sigil_harness::native::GameProfile,
    aeon: &std::path::Path,
) -> Vec<(String, i128)> {
    sigil_harness::native::shape_defines(profile, aeon).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    })
}

/// The header every report shares: what it is, which target it describes, and the
/// define set the target's sources were read under. The defines belong on BOTH
/// reports — `MAX_RING_BUFFER` sizes the RAM regions as surely as it gates the
/// contract walk — so a pasted report always carries its own provenance.
fn print_report_header(kind: &str, label: &str, defines: &[(String, i128)]) {
    println!("{kind}, {label}");
    let ds: Vec<String> = defines.iter().map(|(k, v)| format!("{k}={v}")).collect();
    println!("defines: {}", if ds.is_empty() { "(none)".to_string() } else { ds.join(" ") });
    println!();
}

/// Render the RAM map as a plain, aligned text table (T1). Sizes are byte counts (the
/// "real number"); addresses are `$XXXXXXXX`; `USE%` is used size over region capacity.
fn print_ram_report(rows: &[sigil_frontend_emp::lower::RamRegionRow]) {
    println!(
        "{:<12} {:<11} {:<11} {:<11} {:>7} {:>6} {:>9} {:>6}",
        "REGION", "BASE", "END", "LIMIT", "SIZE", "PAD", "HEADROOM", "USE%"
    );
    for r in rows {
        let cap = r.capacity();
        let pct = if cap > 0 { (r.size as f64) * 100.0 / (cap as f64) } else { 0.0 };
        let name = if r.public { r.name.clone() } else { format!("{} (priv)", r.name) };
        println!(
            "{:<12} ${:08X}  ${:08X}  ${:08X}  {:>7} {:>6} {:>9} {:>5.1}%",
            name,
            r.base,
            r.end(),
            r.limit,
            r.size,
            r.padding,
            r.headroom(),
            pct,
        );
    }
}

/// `--report contracts`: print the whole-corpus contract-closure report for the
/// selected target — the transitive-closure census (`corpus_contracts::analyze_corpus_with`)
/// that the §1 closure, the §6 flag/conditional-result gates, D1b/D1c/D1d, the
/// survives verifier, G5 slot typing, the `[bus.*]` inference tier and the declared
/// `[context.*]` tier all already compute during a build.
///
/// The report is a VIEW, not a second analysis: every number below is a field of the
/// one `ContractReport` the frontend builds. What the report surface adds over a
/// hand-driven walk is the DEFINES — they come from the target's shipping profile, so
/// a shape's real `-D` set (DEBUG, CRASH_REPORT, the game sizing consts) is what the
/// closure sees. A census run against the wrong define set silently analyzes arms the
/// shipped ROM never assembles.
///
/// SCOPE, precisely: the DEFINES are target-accurate, the MODULE SET is not. The walk
/// takes every `.emp` the manifest scan finds, so both games' modules enter one
/// closure under one game's defines. That is the census shape every corpus gate
/// already walks, and narrowing it to `profile.registry` would make this surface
/// disagree with all of them — recorded in the gap ledger rather than changed here.
fn run_contract_report(aeon: &std::path::Path, target: &BuildTarget) {
    let (label, profile) = target.label_and_profile();
    let defines = shape_defines_or_exit(&profile, aeon);
    let (report, _manifest) = corpus_closure_or_exit(aeon, target);
    print_report_header("contract closure", &label, &defines);
    print_contract_report(&report);
}

/// `--report indirect-cost`: what the trusted `jsr (aN) as Type` narrowing is
/// worth to the warn tier, for the selected shape.
///
/// `jsr (aN) as Type` replaces the ⊤ an unbounded indirect call would contribute
/// with the contract type's own clobber set. Nothing proves the procs installed in
/// the dispatch table satisfy that bound, so the narrowing is trusted; the price of
/// withdrawing it is the engine contracts that are written against the narrow
/// answer and would have to be corrected. That price is a MEASUREMENT, and it moves
/// as ordinary object work reaches the dispatch loop, so this prints it on demand
/// rather than anyone recording an integer that goes stale. Nothing gates on it.
///
/// The two firing lists come from the SAME analysis the build gate runs, one under
/// each [`IndirectPolicy`](sigil_frontend_emp::closure::IndirectPolicy), so the
/// report can never describe a different closure than the one that ships.
fn run_indirect_cost_report(aeon: &std::path::Path, target: &BuildTarget) {
    let (label, profile) = target.label_and_profile();
    let defines = shape_defines_or_exit(&profile, aeon);
    let (report, _manifest) = corpus_closure_or_exit(aeon, target);
    print_report_header("indirect-bound cost", &label, &defines);

    let sites = &report.bounded_indirect_sites;
    println!("\n-- bounded `jsr (aN) as Type` dispatch sites ({}): --", sites.len());
    for (proc, ty) in sites {
        println!("  {proc:<32} as {ty}");
    }
    // What this count IS: the sites the corpus walk collected for THIS shape. A
    // site the walk cannot reach is absent from it: a comptime arm this shape's
    // defines discard, and a site inside a splice template the walk does not
    // resolve. So the source text can hold more than this lists, and the honest
    // cross-check is the grep, printed here so a reader runs both rather than
    // trusting one.
    println!(
        "   (sites the corpus walk reached for this shape; cross-check the source with\n    \
         grep -rn --include='*.emp' -E '(jsr|jmp).*\\) +as +[A-Z]' \"$AEON_DIR\"/engine \"$AEON_DIR\"/games\n    \
         a grep hit missing from the list above is a site the walk structurally cannot see)"
    );

    let trusting = report.firings.len();
    let forced = report.firings_unbounded.len();
    println!(
        "\n-- [proc.clobber-undeclared] firings, `as Type` TRUSTED (today's warn tier): {trusting} --"
    );
    println!("-- [proc.clobber-undeclared] firings, ⊤ FORCED at every indirect site: {forced} --");
    for f in &report.firings_unbounded {
        let kind = if f.unbounded {
            "UNBOUNDED".to_string()
        } else if f.transitive {
            format!("transitive {}", f.reg.as_deref().unwrap_or("?"))
        } else {
            format!("direct     {}", f.reg.as_deref().unwrap_or("?"))
        };
        println!("  {:<32} {kind}", f.proc);
    }
    println!(
        "\n-- COST OF THE FLIP: {} engine contract(s) would have to be corrected --",
        forced.saturating_sub(trusting)
    );
}

/// The one closure both `--report contracts` and the build gate run, so a build
/// can never be gated on a different analysis than the report displays. Exits on
/// a scan or interface-bind failure — a half-bound env silently poisons every
/// condition it missed, so it is as fatal here as it is for the report.
fn corpus_closure_or_exit(
    aeon: &std::path::Path,
    target: &BuildTarget,
) -> (
    sigil_frontend_emp::corpus_contracts::ContractReport,
    sigil_frontend_emp::resolve::manifest::Manifest,
) {
    use sigil_frontend_emp::corpus_contracts;

    let (_label, profile) = target.label_and_profile();
    let defines = shape_defines_or_exit(&profile, aeon);
    let manifest = scan_or_exit(aeon);

    let files: Vec<_> = manifest.modules.iter().map(|m| m.file.clone()).collect();
    // The L1 interface env, bound against THIS shape's game (the game RAM module's
    // parent id names it) — so a comptime condition on an `Iface.MEMBER`
    // (`if Game.CAMERA_JUMP_LOCK { }`) selects the arm the shipped ROM assembles
    // instead of landing on `[comptime.unresolved]`. Bind errors are as fatal as
    // scan errors: a half-bound env silently poisons every condition it missed.
    let (iface_env, bind_diags) = corpus_contracts::bind_corpus_interfaces(
        &files,
        &defines,
        profile.game_module_prefix(),
    );
    let bind_errors: Vec<_> = bind_diags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .cloned()
        .collect();
    render_program_diags(&manifest, &bind_errors);
    if !bind_errors.is_empty() {
        process::exit(1);
    }
    // The manifest rides back out so the gate can turn a firing's span into a
    // file:line:col — see `locate_firing`.
    (corpus_contracts::analyze_corpus_with_contracts(&files, &defines, &iface_env), manifest)
}

/// THE BUILD-INTEGRATED CLOSURE GATE (default-ON).
///
/// The corpus closure needs the whole call graph, so it cannot be computed by
/// per-file lowering; it runs here, before the link. That makes the two-tier
/// story complete: per-file declared-contract checks are build diagnostics whose
/// error tier stops the build, and the closure tier stops it too, by this gate.
///
/// RATCHET, not assert-empty. Two families carry a frozen residue that is mostly
/// verifier-model gap rather than loose contract, and the standing ruling bars
/// editing engine code to please a checker; they are pinned instead, both
/// directions. The zero-firing families are asserted empty, with no baseline to
/// go stale.
///
/// Baselines come from `sigil_harness::contract_baseline` — the SAME constants the
/// CI gates assert against. Neither side owns a copy.
///
/// The gate reads the baseline and NOTHING else: `@allow` / `@as_compat` are
/// per-file-tier mechanisms and do not soften a closure finding. A row leaves
/// this gate by being adjudicated into the baseline, or by being fixed.
///
/// `SIGIL_CONTRACTS=0` is the emergency opt-out (aeon's `build.sh` maps
/// `CONTRACTS=0` onto it). It is loud: a skipped gate prints why.
fn run_contract_gate(aeon: &std::path::Path, target: &BuildTarget) {
    use sigil_harness::contract_baseline as bl;

    if std::env::var("SIGIL_CONTRACTS").is_ok_and(|v| v == "0") {
        eprintln!(
            "warning: contract closure gate SKIPPED (SIGIL_CONTRACTS=0). This is the \
             emergency opt-out, the build is not contract-checked."
        );
        return;
    }

    let (report, manifest) = corpus_closure_or_exit(aeon, target);
    let src_index = sigil_frontend_emp::resolve::manifest::SourceIndex::new(&manifest);
    let mut failed = false;

    // Turn the spans the firings already carry into locations. The whole-program
    // contract gate is the phase most in need of one — it runs over a 142-module
    // corpus — and it was the worst-located in the tool: every firing type carries
    // a REAL span, and the gate collapsed them to `(proc, reg)` strings before
    // diffing, so a new firing's only breadcrumb was a proc name to grep for
    // (lens sweep, seat ERR, finding S14). The diff still compares tuples — that
    // is the pin's identity, and it must stay stable — but the MESSAGE now says
    // where each new row is.
    // A diff row renders as "<key> (got N, want M)", so each firing's key is
    // matched as a PREFIX rather than by parsing the row back apart — the row
    // format belongs to `contract_baseline`, and a parser here would be a second
    // copy of it waiting to drift.
    let report_added = |family: &str, d: &bl::BaselineDiff, keyed: &[(String, sigil_span::Span)]| {
        if d.added.is_empty() {
            return;
        }
        eprintln!("  where the NEW {family} firings are:");
        for row in &d.added {
            let loc = keyed
                .iter()
                .find(|(k, _)| row.starts_with(k.as_str()))
                .and_then(|(_, sp)| src_index.locate(*sp));
            match loc {
                Some(loc) => eprintln!("    {loc}: {row}"),
                None => eprintln!("    (no span recorded): {row}"),
            }
        }
    };

    // BLINDNESS FIRST, and it is not a baseline change. If instructions dropped
    // or a comptime condition failed to resolve, the closure UNDER-approximates:
    // firings vanish, the baseline diff reports them as GONE, and the message
    // below would tell the author to delete the pin over an analysis that just
    // went blind — the destructive direction the ratchet exists to prevent. These
    // two must therefore fail with their own message, before any diff runs.
    if report.dropped_instrs != 0 {
        eprintln!(
            "error: the contract closure DROPPED {} instruction(s), the analysis is \
             under-approximating, so every finding below it is unreliable. This is NOT a \
             baseline change; do not adjudicate rows against it.",
            report.dropped_instrs
        );
        for (proc, n) in &report.dropped_by_proc {
            eprintln!("  {proc}: {n}");
        }
        failed = true;
    }
    if !report.comptime_unresolved.is_empty() {
        eprintln!(
            "error: {} comptime condition(s) did not resolve, a poisoned condition \
             discards BOTH arms, so the closure never saw that code. NOT a baseline change.",
            report.comptime_unresolved.len()
        );
        for (m, n, _) in &report.comptime_unresolved {
            eprintln!("  {m}: {n}");
        }
        failed = true;
    }
    if failed {
        eprintln!(
            "error: contract closure gate FAILED, the analysis is blind. Fix the drops \
             or the unresolved conditions; the baselines mean nothing until it can see."
        );
        process::exit(1);
    }

    // The families CI already asserts EMPTY. Gating only the four baselined ones
    // would have made "the closure gates the build" true of a quarter of it.
    let mut empty_gate = |name: &str, n: usize| {
        if n != 0 {
            eprintln!("error: {name}, {n} firing(s); this family is zero-firing by contract.");
            failed = true;
        }
    };
    empty_gate("unresolved callees (missing extern proc?)", report.closure.unresolved_callees.len());
    empty_gate("extern/proc collisions", report.extern_collisions.len());
    empty_gate("[proc.clobber-undeclared] closure firings", report.firings.len());
    empty_gate("[proc.preserves-unverifiable] word-facet firings", report.word_preserve_firings.len());
    empty_gate("[context.escape]/[context.entry-skip]/[context.reacquire]", report.context_firings.len());
    empty_gate("unknown context references", report.unknown_context_refs.len());
    empty_gate("[call.input-undefined] firings", report.input_firings.len());

    let out_got: Vec<(String, String)> =
        report.out_firings.iter().map(|f| (f.proc.clone(), f.reg.clone())).collect();
    let d = bl::diff_out_unverified(&out_got);
    if !d.is_clean() {
        eprintln!("error: {}", bl::adjudication_message("[proc.out-unverified]", &d));
        let spans: Vec<(String, sigil_span::Span)> = report
            .out_firings
            .iter()
            .map(|f| (format!("{} :: out({})", f.proc, f.reg), f.span))
            .collect();
        report_added("[proc.out-unverified]", &d, &spans);
        failed = true;
    }

    // §G4.5 Z80 out-honesty — its OWN frozen array, wired here so `sigil build`
    // enforces the Z80 baseline exactly as the CI test does (the "read by both
    // gates, neither owns a copy" invariant contract_baseline.rs states).
    let z80_out_got: Vec<(String, String)> =
        report.z80_out_firings.iter().map(|f| (f.proc.clone(), f.unit.clone())).collect();
    let d = bl::diff_z80_out_unverified(&z80_out_got);
    if !d.is_clean() {
        eprintln!("error: {}", bl::adjudication_message("[proc.out-unverified] (Z80)", &d));
        failed = true;
    }

    // The in-out exit-side residue — its OWN baseline, wired into the SAME gate as
    // `out` so a new `inout` firing stops the build (the Z80-lane lesson: a new
    // baseline that no gate reads is dark).
    let inout_got: Vec<(String, String)> =
        report.inout_firings.iter().map(|f| (f.proc.clone(), f.reg.clone())).collect();
    let d = bl::diff_inout_unverified(&inout_got);
    if !d.is_clean() {
        eprintln!("error: {}", bl::adjudication_message("[proc.inout-unverified]", &d));
        let spans: Vec<(String, sigil_span::Span)> = report
            .inout_firings
            .iter()
            .map(|f| (format!("{} :: inout({})", f.proc, f.reg), f.span))
            .collect();
        report_added("[proc.inout-unverified]", &d, &spans);
        failed = true;
    }

    let d1c_got: Vec<(String, String, String)> = report
        .live_clobbered_firings
        .iter()
        .map(|f| (f.proc.clone(), f.callee.clone(), f.reg.clone()))
        .collect();
    // D1c is shape-dependent: the debug family assembles code the plain family
    // never sees, so the pin is per-family (measured 20 plain / 25 debug).
    // The profile, not two bools: `diff_d1c` derives BOTH family axes from it
    // (debug, and which game's `Game.SCANLINE_CAPS` this shape compiles against),
    // so neither gate carries its own copy of the classification.
    let d = bl::diff_d1c(&d1c_got, &target.label_and_profile().1);
    if !d.is_clean() {
        eprintln!("error: {}", bl::adjudication_message("[call.live-clobbered] (D1c)", &d));
        let spans: Vec<(String, sigil_span::Span)> = report
            .live_clobbered_firings
            .iter()
            .map(|f| (format!("{} @ {} :: {}", f.proc, f.callee, f.reg), f.span))
            .collect();
        report_added("[call.live-clobbered]", &d, &spans);
        failed = true;
    }

    // Zero-firing families: asserted EMPTY, no baseline. A first firing is a real
    // finding and must stop the build rather than start a pin.
    if !report.context_unsatisfied.is_empty() {
        eprintln!(
            "error: [context.unsatisfied], {} unsatisfied requirement(s); this family is \
             zero-firing by contract, so a firing is a defect, not a baseline candidate:",
            report.context_unsatisfied.len()
        );
        for c in &report.context_unsatisfied {
            eprintln!("  {c:?}");
        }
        failed = true;
    }
    if !report.survives_firings.is_empty() {
        eprintln!(
            "error: [proc.out-cond-survives-unverifiable], {} firing(s); zero-firing by \
             contract:",
            report.survives_firings.len()
        );
        for f in &report.survives_firings {
            eprintln!("  {} :: out({} if {}), {}", f.proc, f.reg, f.cc, f.reason);
        }
        failed = true;
    }

    if failed {
        eprintln!(
            "error: contract closure gate FAILED. Set SIGIL_CONTRACTS=0 (aeon: CONTRACTS=0) \
             to build anyway, that is an emergency hatch, not a fix."
        );
        process::exit(1);
    }
}

/// Render a [`ContractReport`](sigil_frontend_emp::corpus_contracts::ContractReport) as
/// plain text: a proc/extern/contract-type count line, then one section per DIAGNOSTIC
/// FAMILY, each headed by its own count so an empty family still shows its zero. The
/// `[context.*]` tail is one header over several lists (regions, claim sites, then each
/// firing kind), because its counts are one tier's census rather than one lint's.
/// Counts are the report's own vector lengths — nothing is recomputed here.
fn print_contract_report(report: &sigil_frontend_emp::corpus_contracts::ContractReport) {
    println!(
        "procs (incl externs): {}   externs: {}   contract-types: {}",
        report.proc_count, report.extern_count, report.contract_type_count
    );

    println!("\n-- dropped instructions (must be 0): {} --", report.dropped_instrs);
    for (proc, n) in &report.dropped_by_proc {
        println!("  DROPPED {n:>3}  {proc}");
    }

    // The toggle complement of the drop count: a comptime `if` whose condition
    // fails to resolve discards BOTH arms at zero drops, so a lost/misspelled
    // define shows ONLY here.
    println!(
        "\n-- [comptime.unresolved] condition names (must be 0): {} --",
        report.comptime_unresolved.len()
    );
    for (proc, name, _span) in &report.comptime_unresolved {
        println!(
            "  UNRESOLVED  {proc:<28} comptime condition references `{name}`, \
             which the define set does not resolve"
        );
    }

    println!("\n-- extern/proc collisions (§11 Q4): {} --", report.extern_collisions.len());
    for (name, _span) in &report.extern_collisions {
        println!("  COLLISION  {name}  (declared both extern proc and proc)");
    }

    let holes = &report.closure.unresolved_callees;
    println!("\n-- unresolved callees (holes, missing extern proc?): {} --", holes.len());
    for h in holes {
        println!("  HOLE  {h}");
    }

    println!("\n-- [proc.clobber-undeclared] closure firings (§1, {}): --", report.firings.len());
    for f in &report.firings {
        let kind = if f.unbounded {
            "UNBOUNDED".to_string()
        } else if f.transitive {
            format!("transitive {}", f.reg.as_deref().unwrap_or("?"))
        } else {
            format!("direct     {}", f.reg.as_deref().unwrap_or("?"))
        };
        println!("  {:<28} {kind}", f.proc);
    }

    use sigil_frontend_emp::flag_check::FlagFiringKind;
    println!("\n-- flag-result firings (§6, {}): --", report.flag_firings.len());
    for f in &report.flag_firings {
        let kind = match &f.kind {
            FlagFiringKind::Unused => format!("[call.flag-result-unused] {} unconsumed", f.flag),
            FlagFiringKind::InvalidPathRead { reg, cc } => {
                format!("[call.result-invalid-path] {reg} read where !{cc}")
            }
        };
        println!("  {:<28} calls {:<24} {kind}", f.proc, f.callee);
    }

    println!("\n-- [call.input-undefined] firings (D1b, {}): --", report.input_firings.len());
    for f in &report.input_firings {
        println!("  {:<28} calls {:<24} input {} undefined on some path", f.proc, f.callee, f.reg);
    }

    println!(
        "\n-- [call.live-clobbered] firings (D1c, {}): --",
        report.live_clobbered_firings.len()
    );
    for f in &report.live_clobbered_firings {
        println!("  {:<28} calls {:<24} holds {} across clobber", f.proc, f.callee, f.reg);
    }

    println!("\n-- [proc.out-unverified] firings (§G4.5, {}): --", report.out_firings.len());
    for f in &report.out_firings {
        println!("  {:<28} out({}), {}", f.proc, f.reg, f.reason);
    }

    println!("\n-- [proc.inout-unverified] firings ({}): --", report.inout_firings.len());
    for f in &report.inout_firings {
        println!("  {:<28} inout({}), {}", f.proc, f.reg, f.reason);
    }

    println!(
        "\n-- [proc.out-unverified] Z80 firings (§G4.5, {} over {} claim(s)): --",
        report.z80_out_firings.len(),
        report.z80_out_claims.len()
    );
    for f in &report.z80_out_firings {
        println!("  {:<28} out({}), {}", f.proc, f.unit, f.reason);
    }

    println!(
        "\n-- [proc.out-cond-survives-unverifiable] firings ({}): --",
        report.survives_firings.len()
    );
    for f in &report.survives_firings {
        println!("  {:<28} out({} if {}) claims survival, but {}", f.proc, f.reg, f.cc, f.reason);
    }

    println!(
        "\n-- [proc.preserves-unverifiable] word-facet firings (§6, {}): --",
        report.word_preserve_firings.len()
    );
    for (proc, reg, _span) in &report.word_preserve_firings {
        println!("  {proc:<28} preserves({reg}) not provable under the closure oracle");
    }

    println!("\n-- dead-saves (D1d worklist, {}): --", report.dead_saves.len());
    for d in &report.dead_saves {
        println!("  {:<28} {:<4} bracketing {}", d.proc, d.reg, d.callees.join(","));
    }

    println!(
        "\n-- [call.slot-type-mismatch] / [option.unguarded-use] firings (G5, {}): --",
        report.slot_firings.len()
    );
    for f in &report.slot_firings {
        use sigil_frontend_emp::type_slice::FiringKind;
        let found = f.found.as_deref().unwrap_or("an untyped value");
        let id = match f.kind {
            FiringKind::OptionUnguarded => "[option.unguarded-use]",
            FiringKind::SlotType => "[call.slot-type-mismatch]",
        };
        // The option firing carries its own remedy — the whole point of giving it
        // a distinct id is telling the author the ONE thing that fixes it.
        let remedy = match f.kind {
            FiringKind::OptionUnguarded => format!(
                ", the sentinel is never ruled out on this path; guard it \
                 (`cmpi #{}.none, {}` / branch away) then `assume_some! {}, {}`",
                found, f.reg, f.reg, f.expected
            ),
            FiringKind::SlotType => String::new(),
        };
        println!(
            "  {id:<26} {:<28} calls {:<24} slot {} expects {} but found {}{}",
            f.proc, f.callee, f.reg, f.expected, found, remedy
        );
    }

    println!(
        "\n-- [branch.condition-constant] firings ({}): --",
        report.branch_const_firings.len()
    );
    for f in &report.branch_const_firings {
        let dir = if f.always_taken { "ALWAYS taken" } else { "NEVER taken" };
        println!(
            "  {:<28} b{:<3} statically decided ({dir}) @ {}..{}",
            f.proc, f.cc, f.span.start, f.span.end
        );
    }

    println!(
        "\n-- [bus.*] Z80-bus machine-state firings (inference tier, {}): --",
        report.bus_firings.len()
    );
    for f in &report.bus_firings {
        use sigil_frontend_emp::z80_bus::BusFiringKind::*;
        let code = match f.kind {
            DoubleStop => "[bus.double-stop]      (E011)",
            StartWithoutStop => "[bus.start-without-stop] (E008)",
            StoppedAtReturn => "[bus.stopped-at-return]  (E007)",
            VdpWriteUnstopped => "[bus.vdp-write-unstopped](E006)",
            ReleasedAtReturn => "[bus.released-at-return]     ",
        };
        println!("  {:<28} {code} @ {}..{}", f.proc, f.span.start, f.span.end);
    }

    println!(
        "\n-- [context.*] declared machine-state tier: {} region(s), {} claim(s), \
         {} discharged call site(s), {} bracket firing(s), {} unsatisfied requirement(s), \
         {} unknown context reference(s) --",
        report.context_regions.len(),
        report.context_claim_sites.len(),
        report.context_discharged.len(),
        report.context_firings.len(),
        report.context_unsatisfied.len(),
        report.unknown_context_refs.len(),
    );
    if !report.context_regions.is_empty() {
        println!("   regions:");
    }
    for (proc, ctx) in &report.context_regions {
        println!("  {proc:<28} with {ctx}");
    }
    if !report.context_claim_sites.is_empty() {
        println!("   claims:");
    }
    for (proc, kind, ctx) in &report.context_claim_sites {
        println!("  {proc:<28} {kind}({ctx})");
    }
    if !report.context_firings.is_empty() || !report.context_unsatisfied.is_empty() {
        println!("   firings:");
    }
    for f in &report.context_firings {
        use sigil_frontend_emp::context::ContextFiringKind::*;
        let id = match f.kind {
            Escape => "[context.escape]",
            EntrySkip => "[context.entry-skip]",
            Reacquire => "[context.reacquire]",
            RteUndischarged => "[context.rte-undischarged]",
        };
        println!("  {:<28} {id} `{}` @ {}..{}", f.proc, f.ctx, f.span.start, f.span.end);
    }
    for f in &report.context_unsatisfied {
        println!(
            "  {:<28} [context.unsatisfied] `{}` at call to {} @ {}..{}",
            f.proc, f.ctx, f.callee, f.span.start, f.span.end
        );
    }
    for (proc, ctx, span) in &report.unknown_context_refs {
        println!("  {proc:<28} [context.unknown] `{ctx}` @ {}..{}", span.start, span.end);
    }
}

/// Which report `--report <kind>` prints instead of building. Each kind renders a
/// view of data the BUILD already computes for the selected target, so a report and
/// the ROM it describes can never disagree.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ReportKind {
    /// The per-region RAM map (T1).
    Ram,
    /// The whole-corpus contract-closure census.
    Contracts,
    /// What the trusted `jsr (aN) as Type` narrowing costs the warn tier, and the
    /// dispatch sites it ranges over.
    IndirectCost,
}

impl ReportKind {
    /// Parse the `--report` value. Unrecognised is a usage error, never a silent
    /// fallback: a typo must not print the wrong report.
    fn parse(value: &str) -> Result<ReportKind, String> {
        match value {
            "ram" => Ok(ReportKind::Ram),
            "contracts" => Ok(ReportKind::Contracts),
            "indirect-cost" => Ok(ReportKind::IndirectCost),
            other => Err(format!(
                "unknown --report '{other}' (want ram, contracts or indirect-cost)"
            )),
        }
    }
}

/// Which native target `sigil build --native` produces.
enum BuildTarget {
    /// Canonical sonic4 (the pinned driver — the `native_full_rom` gate path).
    Sonic4 { debug: bool },
    /// Off-canonical (the chained driver — the `native_offcanonical_full` gate path).
    Demo { debug: bool },
    ConfigA,
    ConfigB,
    /// The 7th (crash-report-OFF) profile: the sonic4 release shape with no MD
    /// Debugger island and no deb2 symbol appendix — every fault vector routes at
    /// `ReleaseFault`. Owner-ruled 2026-08-04; `build.sh` refuses `CRASH_REPORT=0`
    /// and points here.
    Lean,
    /// Off-canonical DEV shape (Art-streaming P2b Task 7): sonic4 DEBUG + STRESS_EVICT.
    /// UNFROZEN — no golden blob, not a `refreeze` target; a forced-eviction soak
    /// fixture built on demand. Reuses the DEBUG frozen size table (the STRESS_EVICT
    /// clamp is an immediate, not a size change), so the chainer resolves it directly.
    StressEvict,
    /// Off-canonical DEV shape (Art-streaming P2c Task 11): sonic4 DEBUG built against a
    /// UNIQUIFIED act art pool (41 pages). UNFROZEN — no golden, not a `refreeze` target.
    /// Unlike `StressEvict`, the pool inflation CHANGES section sizes past the packed-
    /// placement spread step, so its profile sets `fixture_placement` (greedy pack from
    /// measured sizes; org anchors still held). Built on demand for the acceptance soak.
    StressArt,
}

impl BuildTarget {
    /// The target's display label and its shipping
    /// [`GameProfile`](sigil_harness::native::GameProfile).
    ///
    /// Every report reads its inputs from the profile — the game's own modules, the
    /// comptime `-D` defines the `.emp` sources see — so the mapping lives once here
    /// rather than per report. A report keyed off a different profile than the build
    /// describes a ROM that was never produced.
    fn label_and_profile(&self) -> (String, sigil_harness::native::GameProfile) {
        use sigil_harness::native;
        match self {
            BuildTarget::Sonic4 { debug } => (
                if *debug { "sonic4 debug".to_string() } else { "sonic4 plain".to_string() },
                native::sonic4_profile(*debug),
            ),
            BuildTarget::Demo { debug } => (
                if *debug { "demo debug".to_string() } else { "demo plain".to_string() },
                native::demo_profile(*debug),
            ),
            BuildTarget::ConfigA => ("config_a".to_string(), native::config_a_profile()),
            BuildTarget::ConfigB => ("config_b".to_string(), native::config_b_profile()),
            BuildTarget::Lean => ("lean".to_string(), native::lean_profile()),
            BuildTarget::StressEvict => {
                ("stress_evict".to_string(), native::stress_evict_profile())
            }
            BuildTarget::StressArt => {
                ("stress_art".to_string(), native::stress_art_profile())
            }
        }
    }
}

struct BuildOpts {
    aeon: String,
    output: Option<String>,
    emit_lst: Option<String>,
    target: BuildTarget,
    /// `--report <kind>`: print a report over the selected target and exit, without
    /// building the ROM. `None` builds.
    report: Option<ReportKind>,
    /// `--extra-entry <module>` (repeatable): modules to EVALUATE inside this build's
    /// profile although nothing `use`s them. Each is a dotted module id or a path to
    /// an `.emp` file under the scan root. Only the NAMED module is checked for
    /// byte-neutrality; what its own imports pull in is not.
    extra_entries: Vec<String>,
    /// `--check`: decide every guard against the final placement and stop before
    /// the link. No ROM, no listing, no appendix. `false` builds.
    check: bool,
}

/// Parse `sigil build`'s argument slice. `--aeon <dir>` is required; `-o <path>`,
/// `--emit-lst <path>`, `--game <name>`, `--debug`, `--config-a`, `--config-b`,
/// `--lean` are optional. `--config-a`/`--config-b`/`--lean` fix the whole shape
/// (sonic4 game), so they conflict with `--game`/`--debug`. `--native` is accepted as a no-op (post-flip
/// the native build is the only build).
///
/// `--report <kind>` replaces the build with a report over the same target. The
/// kinds share ONE flag rather than one flag per kind: a report is a view of the
/// build's own data, the set of views grows, and a closed vocabulary behind one flag
/// is the surface that stays legible as it does.
///
/// `--extra-entry <module-id-or-path>` is REPEATABLE: each names a module the build
/// must evaluate although nothing `use`s it, so its module-level `ensure`s run inside
/// the real profile and a false one fails the build with its own message. A NAMED
/// module that would emit is refused by name; the refusal does not reach through that
/// module's own imports, so byte-neutrality is a property of the argument, not of its
/// whole import closure.
///
/// `--check` runs the build's resolve over the same target (so `--game`, `--debug`,
/// the `--config-*` shapes and `--extra-entry` all apply) and stops once every guard
/// is decided. It writes nothing, so a ROM or listing destination beside it is
/// refused like it is beside `--report`, and the two non-building modes refuse each
/// other rather than picking one silently.
fn parse_build_args(args: &[String]) -> Result<BuildOpts, String> {
    let mut aeon: Option<String> = None;
    let mut output: Option<String> = None;
    let mut emit_lst: Option<String> = None;
    let mut game: Option<String> = None;
    let mut debug = false;
    let mut config: Option<char> = None;
    let mut stress_evict = false;
    let mut stress_art = false;
    let mut report: Option<ReportKind> = None;
    let mut extra_entries: Vec<String> = Vec::new();
    let mut check = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--aeon" => aeon = Some(next_value(args, &mut i, "--aeon")?),
            "-o" => output = Some(next_value(args, &mut i, "-o")?),
            "--emit-lst" => emit_lst = Some(next_value(args, &mut i, "--emit-lst")?),
            "--game" => game = Some(next_value(args, &mut i, "--game")?),
            "--native" => {} // accepted as a no-op — native is the only build post-flip
            "--debug" => debug = true,
            "--config-a" => config = Some('a'),
            "--config-b" => config = Some('b'),
            "--lean" => config = Some('l'),
            // Off-canonical DEV shape (Art-streaming P2b Task 7): sonic4 DEBUG with the
            // STRESS_EVICT comptime define = 1 (clamps the residency cache below the pool
            // size, forcing continuous eviction). UNFROZEN — no golden, not a refreeze
            // target; built on demand for the controller's forced-eviction soak.
            "--stress-evict" => stress_evict = true,
            // Off-canonical DEV shape (Art-streaming P2c Task 11): sonic4 DEBUG against a
            // UNIQUIFIED act art pool (41 pages, +tens of KB in the ojz_act_pool section).
            // UNFROZEN — no golden, not a refreeze target. Selects fixture placement (the
            // profile sets `fixture_placement`), plumbed ONLY from build.sh's STRESS_ART
            // path. Fixes the whole shape, so it refuses any shipped-shape selector below.
            "--stress-art" => stress_art = true,
            // Repeatable, order-preserving: one module evaluated per occurrence.
            "--extra-entry" => extra_entries.push(next_value(args, &mut i, "--extra-entry")?),
            "--check" => check = true,
            "--report" => {
                let kind = ReportKind::parse(&next_value(args, &mut i, "--report")?)?;
                if report.is_some_and(|prev| prev != kind) {
                    return Err("--report takes one kind; naming two prints only the last".into());
                }
                report = Some(kind);
            }
            other => return Err(format!("unexpected argument '{other}'")),
        }
        i += 1;
    }

    let aeon = aeon.ok_or("--aeon <dir> is required")?;

    if stress_evict && (config.is_some() || game.is_some() || debug) {
        return Err("--stress-evict fixes the shape (sonic4 debug + STRESS_EVICT); do not combine with --game/--debug/--config-*".into());
    }
    if stress_art && (config.is_some() || game.is_some() || debug || stress_evict) {
        return Err("--stress-art fixes the shape (sonic4 debug + uniquified pool + fixture placement); do not combine with --game/--debug/--config-*/--stress-evict".into());
    }
    let target = match config {
        Some(c) => {
            if game.is_some() || debug {
                return Err("--config-a/--config-b/--lean fix the shape; do not combine with --game/--debug".into());
            }
            match c {
                'a' => BuildTarget::ConfigA,
                'b' => BuildTarget::ConfigB,
                _ => BuildTarget::Lean,
            }
        }
        None if stress_evict => BuildTarget::StressEvict,
        None if stress_art => BuildTarget::StressArt,
        None => match game.as_deref() {
            None | Some("sonic4") => BuildTarget::Sonic4 { debug },
            Some("demo") => BuildTarget::Demo { debug },
            Some(g) => return Err(format!("unknown --game '{g}' (want sonic4 or demo)")),
        },
    };
    // A report prints and exits, so a ROM destination given alongside one would be
    // silently ignored — the caller asked for two different things and gets one.
    if report.is_some() && (output.is_some() || emit_lst.is_some()) {
        return Err("--report prints instead of building; drop -o / --emit-lst".into());
    }
    // An extra entry is evaluated by the BUILD, so pairing it with a report asks for
    // a guard run and gets a print — same reason `-o` is refused above.
    if report.is_some() && !extra_entries.is_empty() {
        return Err("--report prints instead of building; --extra-entry needs a build".into());
    }
    // A check writes no ROM and no listing, so a destination beside it names an
    // artifact that will not exist; and it is not a report, so the two refuse each
    // other instead of one silently winning.
    if check && (output.is_some() || emit_lst.is_some()) {
        return Err("--check decides guards without writing a ROM; drop -o / --emit-lst".into());
    }
    if check && report.is_some() {
        return Err("--check and --report are different runs; pick one".into());
    }
    Ok(BuildOpts { aeon, output, emit_lst, target, report, extra_entries, check })
}

/// Consume the value after a value-taking flag at `args[*i]`, advancing `i`. A
/// missing value is a usage error (the caller renders it).
fn next_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    *i += 1;
    args.get(*i).cloned().ok_or_else(|| format!("{flag} requires a value argument"))
}

/// How much of the warn tier a build prints, from `SIGIL_WARNINGS`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum WarningView {
    /// Print nothing at all.
    Off,
    /// One tally line naming every firing lint id and its count (the default).
    Summary,
    /// The tally line plus one `path:line:col` row per warning.
    Full,
}

impl WarningView {
    /// Read the view from `SIGIL_WARNINGS`.
    fn from_env() -> WarningView {
        WarningView::parse(std::env::var("SIGIL_WARNINGS").ok().as_deref())
    }

    /// Unset or unrecognised reads as [`Summary`](Self::Summary): the tally is
    /// cheap, and a typo'd value must not silently restore the invisibility this
    /// surface exists to end.
    fn parse(value: Option<&str>) -> WarningView {
        match value {
            Some("off") => WarningView::Off,
            Some("full") => WarningView::Full,
            _ => WarningView::Summary,
        }
    }
}

/// The one-line warn-tier tally: how many diagnostics of each severity, then every
/// firing lint id with its count, most-frequent first and ties broken by id so the
/// line is stable build to build (a changed tally means the corpus changed, never
/// the map's iteration order). Returns `None` for an empty tier.
///
/// A diagnostic with no `[id]` prefix tallies as `unclassified`; that bucket is a
/// defect in the lint that emitted it, not a category, and the corpus gate refuses
/// it.
fn warning_summary(warnings: &[sigil_harness::native::BuildWarning]) -> Option<String> {
    if warnings.is_empty() {
        return None;
    }
    let notes = warnings.iter().filter(|w| w.level == sigil_span::Level::Note).count();
    let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    let head = match (warnings.len() - notes, notes) {
        (w, 0) => plural(w, "warning"),
        (0, n) => plural(n, "note"),
        (w, n) => format!("{}, {}", plural(w, "warning"), plural(n, "note")),
    };
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for w in warnings {
        *counts.entry(if w.id.is_empty() { "unclassified" } else { w.id.as_str() }).or_default() +=
            1;
    }
    let mut rows: Vec<(&str, usize)> = counts.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let breakdown = rows.iter().map(|(id, n)| format!("{id} {n}")).collect::<Vec<_>>().join(", ");
    Some(format!("{head}, {breakdown}"))
}

/// The exact stderr lines `report_warnings` emits for `warnings` under `view`.
///
/// Split out from the printing so the surface itself is testable: [`Off`] and an
/// empty tier both yield no lines, [`Summary`] yields the tally plus the pointer to
/// the full view, and [`Full`] yields one located row per warning with the tally
/// LAST, where it survives a scroll.
///
/// [`Off`]: WarningView::Off
/// [`Summary`]: WarningView::Summary
/// [`Full`]: WarningView::Full
fn warning_report_lines(
    view: WarningView,
    warnings: &[sigil_harness::native::BuildWarning],
) -> Vec<String> {
    if view == WarningView::Off {
        return Vec::new();
    }
    let Some(summary) = warning_summary(warnings) else { return Vec::new() };
    let mut lines: Vec<String> = Vec::new();
    if view == WarningView::Full {
        lines.extend(warnings.iter().map(ToString::to_string));
        lines.push(format!("warning: {summary}"));
    } else {
        lines.push(format!("warning: {summary}; SIGIL_WARNINGS=full to list"));
    }
    lines
}

/// Report the warn tier on stderr under the [`WarningView`] the environment
/// selects. Every warn-tier surface of `sigil build` goes through here — the ROM
/// build and every `--report` alike — so one setting governs both. (`sigil emp` /
/// `check` / `test` are single-file report commands that print every diagnostic
/// unconditionally; their job IS the report.)
///
/// The default is a SUMMARY because both extremes fail the same way: a tier that
/// prints nothing is a tier nobody acts on, and a hundred rendered warnings per
/// build is a wall people learn to scroll past. The tally line is bounded by the
/// number of distinct lint ids rather than the number of firings, so a new lint's
/// arrival is legible even when the counts are large.
///
/// A clean build prints nothing: silence means zero, and only zero.
fn report_warnings(warnings: &[sigil_harness::native::BuildWarning]) {
    for line in warning_report_lines(WarningView::from_env(), warnings) {
        eprintln!("{line}");
    }
}

/// `sigil build --check`: decide every `ensure` and every `LinkAssert` of the
/// target against the final post-relaxation placement, then stop. Runs exactly the
/// resolve the full build runs (`native::check_chained`: sound artifacts for a
/// sound-on shape, the AS residual, the `.emp` lowering, the declared chain,
/// `resolve_layout`, `check_link_asserts`, the drift verdict and the inapplicable
/// allowlist) and none of what follows it: no link, no listing, no appendix, no
/// checksum, no ROM. Exit 0 when every decided guard holds; exit 1 with the same
/// rendered guard lines the full build prints when any fails.
///
/// The contract closure gate is NOT run here, and the banner says so: a green
/// check is a statement about guards against placement, not that the game builds.
fn run_check_native(aeon: &std::path::Path, opts: &BuildOpts) {
    use sigil_harness::native;

    let (label, profile) = opts.target.label_and_profile();
    let profile = profile.with_extra_entries(opts.extra_entries.iter().cloned());
    eprintln!(
        "check: {label}: deciding every ensure and LinkAssert against final post-relaxation \
         placement, then stopping before the link: a green check proves nothing about region \
         budget or overlap, image bounds, the checksum or the contract closure gate, and is not \
         a statement that the game builds"
    );
    if profile.sound_on {
        eprintln!(
            "check: {label}: emit_generated writes the sound artifacts into {} (a precondition \
             of the .emp build), the one write this run makes",
            aeon.display()
        );
    }
    let native::CheckReport { guards, warnings } = match native::check_chained(aeon, &profile) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("error: native check ({label}): {err}");
            process::exit(1);
        }
    };
    report_warnings(&warnings);
    println!(
        "checked: {label}: {} ensure verdict(s) at comptime, {} LinkAssert(s) decided at link, \
         {} LinkAssert(s) inapplicable (extern not defined in this link, allowlisted)",
        guards.comptime_guards, guards.link_asserts_decided, guards.link_asserts_inapplicable
    );
}

/// The `--native` build. Reproduces the exact steps the native gates bank: get the
/// assembled ROM + sigil-canonical listing from the target's driver (pinned for
/// canonical sonic4, the declared-order chainer for the off-canonical shapes), then
/// the `convsym`+`fixheader` deb2 appendix over that same (rom, listing) — so the
/// full file is byte-identical to `build_native_full_file`/`build_full_file_chained`
/// and a green gate vouches for the bytes `build.sh` ships. `--emit-lst` drops the
/// sigil-canonical `.lst` (the `.lst`-consumer drop-in). Prints `crc=<crc32>
/// len=<bytes>` for the build log / provenance check.
///
/// SHAPE SPLIT (the crash-report axis, owner-ruled 2026-08-04 — SUPERSEDES the
/// review-item-29 release strip): the appendix follows the MD Debugger island, so it
/// runs whenever `debug || crash_report` — i.e. in every shape except the opt-in
/// `--lean`, which writes the assembled ROM verbatim. `build_native_full_file` /
/// `build_full_file_chained` model the same rule off `GameProfile::crash_report`.
fn run_build_native(aeon: &std::path::Path, opts: &BuildOpts) {
    use sigil_harness::native;

    // The closure gate runs BEFORE the build: a contract failure should cost a
    // second, not a full link, and it must not leave a fresh artifact on disk
    // that looks like it passed.
    run_contract_gate(aeon, &opts.target);

    // The LABEL comes from the same place the reports read it, so a build and a
    // report over one target can never name it differently.
    let label = opts.target.label_and_profile().0;
    // `--extra-entry` rides the PROFILE, so every target honours it through one
    // spelling. Canonical sonic4 keeps its own driver entry point when the list is
    // empty — the shipped path stays the byte bar's path, untouched — and routes
    // through the chainer (which is what that driver delegates to for a `Frozen`
    // size source anyway) when an extra entry has to reach `build_emp`.
    let extra = &opts.extra_entries;
    let with_extra = |p: native::GameProfile| p.with_extra_entries(extra.iter().cloned());
    // (rom, listing) from the target's driver + the target's appendix floor + shape.
    let (debug, floor, built) = match &opts.target {
        // Canonical sonic4 → the PINNED driver (the `native_full_rom` gate path).
        BuildTarget::Sonic4 { debug } if extra.is_empty() => (
            *debug,
            native::SONIC4_APPENDIX_FLOOR,
            native::build_native_rom_with_listing(aeon, *debug),
        ),
        BuildTarget::Sonic4 { debug } => (
            *debug,
            native::SONIC4_APPENDIX_FLOOR,
            native::build_rom_chained_with_listing(
                aeon,
                &with_extra(native::sonic4_profile(*debug)),
            ),
        ),
        // Off-canonical → the declared-order CHAINER (the `native_offcanonical_full` path).
        BuildTarget::Demo { debug } => (
            *debug,
            native::DEMO_APPENDIX_FLOOR,
            native::build_rom_chained_with_listing(aeon, &with_extra(native::demo_profile(*debug))),
        ),
        BuildTarget::ConfigA => (
            true,
            native::SONIC4_APPENDIX_FLOOR,
            native::build_rom_chained_with_listing(aeon, &with_extra(native::config_a_profile())),
        ),
        BuildTarget::ConfigB => (
            false,
            native::SONIC4_APPENDIX_FLOOR,
            native::build_rom_chained_with_listing(aeon, &with_extra(native::config_b_profile())),
        ),
        BuildTarget::Lean => (
            false,
            native::SONIC4_APPENDIX_FLOOR,
            native::build_rom_chained_with_listing(aeon, &with_extra(native::lean_profile())),
        ),
        // Off-canonical DEV soak shape — the declared-order CHAINER, debug appendix.
        BuildTarget::StressEvict => (
            true,
            native::SONIC4_APPENDIX_FLOOR,
            native::build_rom_chained_with_listing(
                aeon,
                &with_extra(native::stress_evict_profile()),
            ),
        ),
        // Off-canonical DEV fixture (uniquified pool) — CHAINER with fixture placement.
        BuildTarget::StressArt => (
            true,
            native::SONIC4_APPENDIX_FLOOR,
            native::build_rom_chained_with_listing(aeon, &with_extra(native::stress_art_profile())),
        ),
    };
    let native::RomBuild { rom, listing, warnings } = match built {
        Ok(build) => build,
        Err(err) => {
            eprintln!("error: native build ({label}): {err}");
            process::exit(1);
        }
    };
    report_warnings(&warnings);

    // The deb2 symbol appendix over the SAME (rom, listing) — byte-identical to the
    // full-file gate function (which folds the checksum in `emit_rom` then appends).
    //
    // THE CRASH-REPORT AXIS (owner-ruled 2026-08-04): the appendix is the MD
    // Debugger's symbol table, and the debugger is a DIAGNOSTIC — a player's crash
    // has to name the code it died in. So it ships in DEBUG and in RELEASE; the
    // ~29.7 KB is 7.2% of a ROM that is itself 9% of a 4 MB cart. Only the opt-in
    // LEAN shape (no island, faults route at ReleaseFault) writes the assembled ROM
    // verbatim — same length, same header (`emit_rom` already folded the checksum
    // over exactly these bytes, so no re-fix is needed).
    //
    // THE SHAPE ANSWER IS DERIVED, NOT NAMED HERE. Spelling it as a target match
    // (`!matches!(opts.target, Lean)`) made this a THIRD hand-maintained copy of one
    // fact, and a copy that a new no-island target would silently fall out of.
    // `declares_error_handler_island` reconciles the profile's crash-report axis with
    // the module list its build is handed and refuses when they disagree, so this
    // site and `append_deb2_appendix`'s blob-label check read the same answer.
    let island = match native::declares_error_handler_island(&opts.target.label_and_profile().1) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("error: native build ({label}) fault-handler shape: {err}");
            process::exit(1);
        }
    };
    let full = if island {
        match native::append_deb2_appendix(aeon, &rom, &listing, debug, floor, island) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("error: native build ({label}) appendix: {err}");
                process::exit(1);
            }
        }
    } else {
        rom
    };

    // The sigil-canonical listing (the `.lst`-consumer drop-in), if requested, opens with
    // the source digest, which names the FULL shipped file above, appendix included. So
    // the listing is written after the ROM is final. The appendix is built from the
    // in-memory `listing`, never from the `.lst` text, so the order moves no ROM byte.
    // The digest is rendered before either artifact is written, so a read set that
    // cannot be stated (a file that changed while the build read it) leaves neither.
    let digest = match &opts.emit_lst {
        Some(_) => match lst_source_digest(aeon, opts, &full) {
            Ok(section) => Some(section),
            Err(err) => {
                eprintln!("error: native build ({label}) source digest: {err}");
                process::exit(1);
            }
        },
        None => None,
    };
    if let Some(out_path) = &opts.output {
        if let Err(err) = install_artifact(out_path, &full) {
            eprintln!("error: cannot write {out_path}: {err}");
            process::exit(1);
        }
    }
    if let (Some(lst_path), Some(section)) = (&opts.emit_lst, digest) {
        let text = format!("{section}{}", sigil_link::emit_listing(&listing));
        if let Err(err) = install_artifact(lst_path, text.as_bytes()) {
            eprintln!("error: cannot write {lst_path}: {err}");
            process::exit(1);
        }
    }
    println!("built: {label} native ROM, crc={:08x} len={}", native::crc32(&full), full.len());
}

/// The `.lst` source digest for this build: the read set the recorder captured, the
/// build configuration, and the identity of the ROM about to be written to `-o`.
fn lst_source_digest(
    aeon: &std::path::Path,
    opts: &BuildOpts,
    full: &[u8],
) -> Result<String, String> {
    use sigil_harness::{native, source_digest};

    let profile = opts.target.label_and_profile().1;
    // Everything this reads happens before the snapshot below, so the digest names it.
    let defines = native::shape_defines(&profile, aeon)?;
    let (target, game) = digest_target(&opts.target);
    let snapshot = sigil_span::read_set::snapshot();
    let digest = source_digest::source_digest(
        aeon,
        &source_digest::AssemblerIdentity {
            version: env!("CARGO_PKG_VERSION"),
            revision: env!("SIGIL_REVISION"),
            tree_state: env!("SIGIL_TREE_STATE"),
        },
        &source_digest::DigestShape {
            target,
            game,
            debug: profile.debug,
            extra_entries: &opts.extra_entries,
        },
        defines,
        full,
        opts.output.as_deref().map(std::path::Path::new),
        &snapshot,
    )?;
    sigil_link::emit_source_digest(&digest)
}

/// The digest's `target=` and `game=` for a build target: the flag that selects it
/// and the game it builds. Exhaustive, so a new target has to name both.
fn digest_target(target: &BuildTarget) -> (&'static str, &'static str) {
    match target {
        BuildTarget::Sonic4 { .. } => ("sonic4", "sonic4"),
        BuildTarget::Demo { .. } => ("demo", "demo"),
        BuildTarget::ConfigA => ("config-a", "sonic4"),
        BuildTarget::ConfigB => ("config-b", "sonic4"),
        BuildTarget::Lean => ("lean", "sonic4"),
        BuildTarget::StressEvict => ("stress-evict", "sonic4"),
        BuildTarget::StressArt => ("stress-art", "sonic4"),
    }
}

#[cfg(test)]
mod help_gates {
    //! The help text and the command line are one table, and these hold the
    //! parts of that relationship the type system cannot.
    //!
    //! A help text is a second copy of an interface, and a second copy drifts.
    //! [`super::ENTRIES`] removes the copy for the command LIST (dispatch and
    //! help read the same rows, so a seventh command cannot be dispatched
    //! without being listed), and these gates cover what is left: that the list
    //! renders every row, that each row's usage is about that row, that no flag
    //! a row accepts is missing from its usage, and that dispatch still goes
    //! through the table instead of a hand-written match.
    //!
    //! They assert RELATIONSHIPS between the table, the source, and the
    //! rendered text, never a golden copy of the text, so rewording help is
    //! free and dropping a command from it is not.

    use super::{Entry, ENTRIES};
    use std::collections::BTreeSet;

    /// This binary's own source, read at compile time. The flag and dispatch
    /// gates ask questions about the code that no runtime value answers.
    const SOURCE: &str = include_str!("main.rs");

    /// The source with its own test modules removed, so a gate cannot be
    /// satisfied (or tripped) by test code.
    fn shipping_source() -> &'static str {
        let marker = "\n#[cfg(test)]\nmod help_gates {";
        let end = SOURCE.find(marker).expect("the test modules start with a cfg(test) marker");
        &SOURCE[..end]
    }

    /// The text of `fn <name>` in this source, from its signature to its
    /// closing brace at column zero.
    fn fn_body(name: &str) -> &'static str {
        let src = shipping_source();
        let needle = format!("\nfn {name}(");
        let start = src
            .find(&needle)
            .unwrap_or_else(|| panic!("no `fn {name}` in the shipping source"));
        let rest = &src[start + 1..];
        let end = rest.find("\n}\n").map(|i| i + 3).unwrap_or(rest.len());
        &rest[..end]
    }

    /// Every flag literal in the match arms of `text`: a line that begins with a
    /// string literal and carries a `=>` is an argument-match arm, and every
    /// `-`-prefixed literal on it is a flag that arm accepts.
    fn flag_arms(text: &str) -> BTreeSet<String> {
        let mut flags = BTreeSet::new();
        for line in text.lines() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('"') || !trimmed.contains("=>") {
                continue;
            }
            let mut rest = trimmed;
            while let Some(open) = rest.find('"') {
                let after = &rest[open + 1..];
                let Some(close) = after.find('"') else { break };
                let literal = &after[..close];
                if literal.starts_with('-') {
                    flags.insert(literal.to_string());
                }
                rest = &after[close + 1..];
            }
        }
        flags
    }

    /// The top-level list names every entry point, by label and by summary.
    /// A row added to the table appears here without anyone writing a line.
    #[test]
    fn top_level_help_lists_every_entry() {
        let help = super::top_level_help();
        assert!(!ENTRIES.is_empty(), "the entry table is empty");
        for e in ENTRIES {
            assert!(help.contains(e.label), "top-level help omits label `{}`:\n{help}", e.label);
            assert!(
                help.contains(e.summary),
                "top-level help omits the summary of `{}`:\n{help}",
                e.label
            );
        }
        assert!(
            help.contains("--help"),
            "top-level help must say how to reach a command's own help:\n{help}"
        );
    }

    /// The command each usage line invokes: the token after `sigil ` on it.
    ///
    /// A whole-text substring search cannot answer this question, because the
    /// commands are substrings of the arguments they take: a `sigil emp` usage
    /// line that had lost its `emp` still contains `emp`, inside `<input.emp>`.
    /// The token after `sigil` is the thing a reader would type.
    fn usage_commands<'a>(usage: &[&'a str]) -> BTreeSet<&'a str> {
        let mut out = BTreeSet::new();
        for line in usage {
            let Some(at) = line.find("sigil ") else { continue };
            if let Some(token) = line[at + "sigil ".len()..].split_whitespace().next() {
                out.insert(token);
            }
        }
        out
    }

    /// Each row's usage text is about that row: it opens with a usage line, and
    /// every word that selects the row is the command on one of those lines.
    /// Catches a usage string copied from a sibling and left naming the sibling.
    #[test]
    fn every_entry_usage_names_its_own_entry() {
        for e in ENTRIES {
            let usage = e.usage.join("\n");
            assert!(!e.summary.is_empty(), "`{}` has no summary", e.label);
            assert!(
                e.usage.first().is_some_and(|l| l.starts_with("usage: sigil")),
                "`{}` usage does not open with a usage line: {usage}",
                e.label
            );
            let commands = usage_commands(e.usage);
            assert!(
                commands.contains(e.label),
                "`{}` usage never invokes the entry point (it invokes {commands:?}): {usage}",
                e.label
            );
            for word in e.words {
                assert!(
                    commands.contains(word),
                    "`{}` usage never invokes its selecting word `{word}` \
                     (it invokes {commands:?}): {usage}",
                    e.label
                );
            }
        }
    }

    /// No two rows answer to the same word, so `entry_for` cannot depend on
    /// table order.
    #[test]
    fn entry_words_are_unique() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for e in ENTRIES {
            for word in e.words {
                assert!(seen.insert(word), "two entry points answer to `{word}`");
            }
        }
        assert!(!seen.is_empty(), "no entry point has a selecting word");
    }

    /// Every flag an entry point's parser ACCEPTS appears in that entry point's
    /// usage, and every argument-match arm in the shipping source belongs to
    /// some entry point's parser.
    ///
    /// This is the gate for the defect it closes: `--map` and `--deny-todo` were
    /// accepted by `sigil emp` and named in no usage line at all, so the only
    /// way to learn they existed was to read the source. It is a relationship
    /// between the parser and the text, so a flag added tomorrow fails here
    /// until its usage line exists, and no wording change can fail it.
    #[test]
    fn usage_names_every_accepted_flag() {
        let mut attributed: BTreeSet<String> = BTreeSet::new();
        for e in ENTRIES {
            let body = fn_body(e.flags_fn);
            assert!(
                body.contains("fn ") && body.len() > 100,
                "the slice for `{}` is too small to be a function body: {body}",
                e.flags_fn
            );
            let usage = e.usage.join("\n");
            for flag in flag_arms(body) {
                assert!(
                    usage.contains(&flag),
                    "`{}` accepts `{flag}` and its usage never names it:\n{usage}",
                    e.label
                );
                attributed.insert(flag);
            }
        }
        // The instrument found something: if the scanner stopped matching, this
        // is what says so rather than an empty sweep reading as a clean one.
        assert!(
            attributed.len() >= 10,
            "the flag scanner found only {} flags across the whole command line, \
             which is fewer than this binary is known to accept",
            attributed.len()
        );
        // And it found everything: an argument parser reachable from no entry
        // point accepts flags nothing documents.
        let all = flag_arms(shipping_source());
        let orphans: Vec<&String> = all.difference(&attributed).collect();
        assert!(
            orphans.is_empty(),
            "flags accepted by no entry point's parser, so named in no usage: {orphans:?}"
        );
    }

    /// Dispatch reads the table. A hand-written arm in `main` would be an entry
    /// point the help text knows nothing about, which is the whole defect class
    /// the table exists to prevent, so the arm shape itself is refused.
    #[test]
    fn dispatch_reads_only_the_entry_table() {
        let body = fn_body("main");
        // Controls: the slice really is `main`, and it really does dispatch
        // through the table, so a failure below is about the code and not about
        // the slicer having grabbed the wrong text.
        assert!(body.contains("entry_for("), "main does not look up the entry table: {body}");
        assert!(body.contains("(entry.run)("), "main does not run a table row: {body}");
        assert!(
            !body.contains("Some(\""),
            "main dispatches on a literal instead of the entry table, so that entry point \
             appears in no help text:\n{body}"
        );
    }

    /// The help surface carries no em dash or en dash, which is an owner ruling
    /// about product text. Checked over the rendered pages rather than the table
    /// so a dash introduced by a renderer counts too.
    #[test]
    fn help_text_uses_no_dashes() {
        let mut pages = vec![super::top_level_help()];
        pages.extend(ENTRIES.iter().map(super::entry_help));
        assert_eq!(pages.len(), ENTRIES.len() + 1);
        for page in pages {
            assert!(!page.contains('\u{2014}'), "em dash in help text:\n{page}");
            assert!(!page.contains('\u{2013}'), "en dash in help text:\n{page}");
        }
    }

    /// `entry_for` answers for every word in the table and for nothing else, and
    /// the bare-file row is the one with no word.
    #[test]
    fn entry_lookup_matches_the_table() {
        for e in ENTRIES {
            for word in e.words {
                let found: &Entry = super::entry_for(word).expect("a table word resolves");
                assert_eq!(found.label, e.label, "`{word}` resolved to the wrong entry point");
            }
        }
        assert!(super::entry_for("input.asm").is_none(), "a filename selected a command");
        assert!(super::entry_for("--help").is_none(), "help is not a dispatch word");
        assert!(super::bare_entry().words.is_empty());
    }

    /// Every conventional way of asking is a help word, and a flag that is not
    /// a request for help is not.
    #[test]
    fn help_words_are_the_conventional_three() {
        for word in ["--help", "-h", "help"] {
            assert!(super::is_help_word(word), "`{word}` is not treated as a request for help");
        }
        for word in ["--hex", "-o", "helper", "build"] {
            assert!(!super::is_help_word(word), "`{word}` is not a request for help");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// `compile_emp` must resolve an `embed(...)` in the source against the
    /// file's own directory (the include-root the CLI supplies) and lower it to
    /// the embedded bytes — the end-to-end proof that the production emp path
    /// wires `include_root` (Plan 5's sandbox is otherwise `[sandbox.no-root]`).
    #[test]
    fn compile_emp_resolves_embed_against_source_dir() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
        let src = std::fs::read_to_string(dir.join("prog.emp")).expect("read prog.emp");
        let (image, diags) = crate::compile_emp(&src, Some(&dir), &[]);
        assert!(
            diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "unexpected error diagnostics: {diags:?}"
        );
        let blob = std::fs::read(dir.join("blob.bin")).expect("read blob.bin");
        assert_eq!(image.expect("image bytes"), blob);
    }

    /// The `-D` value parser's accepted forms (decimal incl. negative, `$hex`,
    /// `0x`/`0X` hex) and its refusals (overflow, garbage, empty, bare
    /// prefixes). `parse_define` itself `process::exit(2)`s on a `None`, so the
    /// pure int parser is the unit-testable seam.
    #[test]
    fn parse_define_int_accepts_all_documented_forms() {
        assert_eq!(crate::parse_define_int("42"), Some(42));
        assert_eq!(crate::parse_define_int("-7"), Some(-7));
        assert_eq!(crate::parse_define_int("$FF"), Some(0xFF));
        assert_eq!(crate::parse_define_int("$deadBEEF"), Some(0xDEAD_BEEF));
        assert_eq!(crate::parse_define_int("0x10"), Some(0x10));
        assert_eq!(crate::parse_define_int("0X10"), Some(0x10));
        assert_eq!(crate::parse_define_int("0"), Some(0));
    }

    /// `sigil build`'s native flag grammar: `--aeon` required; the target derives
    /// from `--game`/`--debug`/`--config-*`; `--config-*` conflicts with
    /// `--game`/`--debug`; unknown games are refused. Locks the flip build's CLI.
    #[test]
    fn parse_build_args_native_target_selection() {
        use crate::BuildTarget;
        let s = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect::<Vec<_>>();

        // Default (no --game) → canonical sonic4 plain. `--native` is an accepted no-op.
        let o = crate::parse_build_args(&s(&["--aeon", "x", "--native"])).unwrap();
        assert!(matches!(o.target, BuildTarget::Sonic4 { debug: false }));

        // --game sonic4 --debug → sonic4 debug.
        let o = crate::parse_build_args(&s(&["--aeon", "x", "--native", "--game", "sonic4", "--debug"]))
            .unwrap();
        assert!(matches!(o.target, BuildTarget::Sonic4 { debug: true }));

        // --game demo --debug → demo debug, with -o / --emit-lst captured.
        let o = crate::parse_build_args(&s(&[
            "--aeon", "x", "--native", "--game", "demo", "--debug", "-o", "r.bin", "--emit-lst", "r.lst",
        ]))
        .unwrap();
        assert!(matches!(o.target, BuildTarget::Demo { debug: true }));
        assert_eq!(o.output.as_deref(), Some("r.bin"));
        assert_eq!(o.emit_lst.as_deref(), Some("r.lst"));

        // --config-a / --config-b / --lean select those shapes.
        assert!(matches!(
            crate::parse_build_args(&s(&["--aeon", "x", "--native", "--config-a"])).unwrap().target,
            BuildTarget::ConfigA
        ));
        assert!(matches!(
            crate::parse_build_args(&s(&["--aeon", "x", "--native", "--config-b"])).unwrap().target,
            BuildTarget::ConfigB
        ));
        assert!(matches!(
            crate::parse_build_args(&s(&["--aeon", "x", "--native", "--lean"])).unwrap().target,
            BuildTarget::Lean
        ));

        // Refusals: missing --aeon, config+game conflict, config+debug conflict, unknown game.
        // `--lean` fixes the whole shape exactly as --config-a/-b do, so it conflicts the same way.
        assert!(crate::parse_build_args(&s(&["--native"])).is_err());
        assert!(crate::parse_build_args(&s(&["--aeon", "x", "--config-a", "--game", "demo"])).is_err());
        assert!(crate::parse_build_args(&s(&["--aeon", "x", "--config-b", "--debug"])).is_err());
        assert!(crate::parse_build_args(&s(&["--aeon", "x", "--lean", "--debug"])).is_err());
        assert!(crate::parse_build_args(&s(&["--aeon", "x", "--lean", "--game", "demo"])).is_err());
        assert!(crate::parse_build_args(&s(&["--aeon", "x", "--game", "genesis"])).is_err());
    }

    /// `--check` rides every target selector and `--extra-entry`, and refuses the
    /// destinations it would never write and the report it is not.
    #[test]
    fn parse_build_args_check_grammar() {
        use crate::BuildTarget;
        let s = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let o = crate::parse_build_args(&s(&["--aeon", "x", "--check"])).unwrap();
        assert!(o.check);
        assert!(matches!(o.target, BuildTarget::Sonic4 { debug: false }));
        let o = crate::parse_build_args(&s(&["--aeon", "x", "--game", "demo", "--check"])).unwrap();
        assert!(o.check);
        assert!(matches!(o.target, BuildTarget::Demo { debug: false }));
        let o = crate::parse_build_args(&s(&[
            "--aeon", "x", "--check", "--game", "demo", "--debug", "--extra-entry", "games.demo.constants",
        ]))
        .unwrap();
        assert!(o.check);
        assert!(matches!(o.target, BuildTarget::Demo { debug: true }));
        assert_eq!(o.extra_entries, vec!["games.demo.constants".to_string()]);
        let o = crate::parse_build_args(&s(&["--aeon", "x", "--config-b", "--check"])).unwrap();
        assert!(o.check);
        assert!(matches!(o.target, BuildTarget::ConfigB));
        assert!(!crate::parse_build_args(&s(&["--aeon", "x"])).unwrap().check);

        let refused = |args: &[&str], flag: &str| {
            let err = match crate::parse_build_args(&s(args)) {
                Err(e) => e,
                Ok(_) => panic!("{args:?} must be refused"),
            };
            assert!(err.contains("--check") && err.contains(flag), "{args:?}: {err}");
        };
        refused(&["--aeon", "x", "--check", "-o", "r.bin"], "-o");
        refused(&["--aeon", "x", "--check", "--emit-lst", "r.lst"], "--emit-lst");
        refused(&["--aeon", "x", "--check", "--report", "ram"], "--report");
    }

    /// `--extra-entry` is REPEATABLE and order-preserving (one module evaluated per
    /// occurrence), takes a value, and is refused alongside `--report` — which does
    /// not build, so it could only ignore the modules it was asked to evaluate.
    #[test]
    fn parse_build_args_collects_repeated_extra_entries() {
        let s = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let o = crate::parse_build_args(&s(&[
            "--aeon",
            "x",
            "--extra-entry",
            "games.a.one",
            "--extra-entry",
            "b/two.emp",
        ]))
        .unwrap();
        assert_eq!(o.extra_entries, vec!["games.a.one".to_string(), "b/two.emp".to_string()]);

        assert!(crate::parse_build_args(&s(&["--aeon", "x"])).unwrap().extra_entries.is_empty());
        assert!(crate::parse_build_args(&s(&["--aeon", "x", "--extra-entry"])).is_err());
        assert!(crate::parse_build_args(&s(&[
            "--aeon",
            "x",
            "--report",
            "ram",
            "--extra-entry",
            "games.a.one"
        ]))
        .is_err());
    }

    #[test]
    fn parse_define_int_rejects_malformed_input() {
        // Overflow: one past i128::MAX.
        assert_eq!(crate::parse_define_int("170141183460469231731687303715884105728"), None);
        assert_eq!(crate::parse_define_int("$100000000000000000000000000000000"), None);
        // Garbage, wrong-radix digits, empty, bare prefixes.
        assert_eq!(crate::parse_define_int("banana"), None);
        assert_eq!(crate::parse_define_int("$XYZ"), None);
        assert_eq!(crate::parse_define_int("0xZZ"), None);
        assert_eq!(crate::parse_define_int(""), None);
        assert_eq!(crate::parse_define_int("$"), None);
        assert_eq!(crate::parse_define_int("0x"), None);
    }

    /// The warn-tier tally line: severity head, then every firing id with its
    /// count, most-frequent first and ties by id. An empty tier yields `None`, so
    /// a clean build says nothing at all.
    #[test]
    fn warning_summary_tallies_by_id_most_frequent_first() {
        use sigil_harness::native::BuildWarning;
        let w = |level, id: &str| BuildWarning {
            level,
            id: id.to_string(),
            location: None,
            message: format!("[{id}] whatever"),
            primary: sigil_span::Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
        };
        let warn = sigil_span::Level::Warning;

        assert_eq!(crate::warning_summary(&[]), None, "a clean build says nothing");

        // `b.b` twice, `a.a` and `c.c` once: count DESCENDING, then id ascending.
        let ws = [w(warn, "c.c"), w(warn, "b.b"), w(warn, "a.a"), w(warn, "b.b")];
        assert_eq!(crate::warning_summary(&ws).unwrap(), "4 warnings, b.b 2, a.a 1, c.c 1");

        // A message with no `[id]` prefix is not a category — it shows as the
        // defect it is.
        let bare = BuildWarning {
            level: warn,
            id: String::new(),
            location: None,
            message: "no bracket".into(),
            primary: sigil_span::Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
        };
        assert_eq!(crate::warning_summary(&[bare]).unwrap(), "1 warning, unclassified 1");
    }

    /// The Note tier counts and renders SEPARATELY from warnings. The corpus fires
    /// no notes, so only a unit test can hold this arm: the next `Level::Note`
    /// anyone adds is visible the first time it fires.
    #[test]
    fn warning_summary_counts_notes_apart_from_warnings() {
        use sigil_harness::native::BuildWarning;
        let d = |level, id: &str| BuildWarning {
            level,
            id: id.to_string(),
            location: None,
            message: format!("[{id}] whatever"),
            primary: sigil_span::Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
        };
        let (warn, note) = (sigil_span::Level::Warning, sigil_span::Level::Note);

        assert_eq!(
            crate::warning_summary(&[d(warn, "a.a"), d(note, "b.b")]).unwrap(),
            "1 warning, 1 note, a.a 1, b.b 1"
        );
        assert_eq!(crate::warning_summary(&[d(note, "b.b")]).unwrap(), "1 note, b.b 1");
    }

    /// The rendered surface, per view. [`WarningView::Full`] emits one located row
    /// per warning and puts the tally LAST; [`WarningView::Summary`] emits the
    /// tally alone plus the pointer to the full view; [`WarningView::Off`] and an
    /// empty tier emit nothing.
    ///
    /// NOT VACUOUS: this is the only assertion over what the build actually PRINTS.
    /// `warning_summary` alone would still pass with the printer unwired.
    #[test]
    fn warning_report_lines_render_each_view() {
        use crate::{warning_report_lines as lines, WarningView};
        use sigil_harness::native::BuildWarning;
        let w = |id: &str, loc: Option<&str>| BuildWarning {
            level: sigil_span::Level::Warning,
            id: id.to_string(),
            location: loc.map(str::to_string),
            message: format!("[{id}] whatever"),
            primary: sigil_span::Span {
                source: sigil_span::SourceId(0),
                start: 0,
                end: 0,
            },
        };
        let ws = [w("a.a", Some("x.emp:1:2")), w("a.a", None)];

        assert_eq!(lines(WarningView::Off, &ws), Vec::<String>::new());
        assert_eq!(lines(WarningView::Summary, &[]), Vec::<String>::new());
        assert_eq!(lines(WarningView::Full, &[]), Vec::<String>::new());

        assert_eq!(
            lines(WarningView::Summary, &ws),
            ["warning: 2 warnings, a.a 2; SIGIL_WARNINGS=full to list"]
        );
        assert_eq!(
            lines(WarningView::Full, &ws),
            [
                "x.emp:1:2: warning: [a.a] whatever",
                "warning: [a.a] whatever",
                "warning: 2 warnings, a.a 2",
            ]
        );
    }

    /// `SIGIL_WARNINGS` selects the view. An unset or MISSPELLED value reads as
    /// `summary`, never as `off`: a typo must not silently restore the invisibility
    /// this surface exists to end.
    #[test]
    fn warning_view_defaults_to_summary_on_anything_unrecognised() {
        use crate::WarningView;
        assert_eq!(WarningView::parse(Some("off")), WarningView::Off);
        assert_eq!(WarningView::parse(Some("full")), WarningView::Full);
        assert_eq!(WarningView::parse(Some("summary")), WarningView::Summary);
        assert_eq!(WarningView::parse(Some("ful")), WarningView::Summary);
        assert_eq!(WarningView::parse(Some("OFF")), WarningView::Summary);
        assert_eq!(WarningView::parse(Some("")), WarningView::Summary);
        assert_eq!(WarningView::parse(None), WarningView::Summary);
    }
}
