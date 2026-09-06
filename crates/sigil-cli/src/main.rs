//! sigil-cli: the `sigil` command-line assembler binary.
//!
//! Usage: `sigil <input.asm> [-o <output.bin>] [--hex]`
//!        `sigil parse <input.emp>`
//!        `sigil emp <input.emp> [-o <output.bin>] [--hex]`
//!        `sigil build --aeon <dir> [-o <output.bin>] [--emit-lst <lst>] [--game ...] [--debug]`
//!        `sigil --version` / `sigil -V`
//!
//! Assembles the given Z80 source file. Writes the binary image to the path
//! given by `-o` (if supplied). When `--hex` is passed, prints the output
//! bytes as uppercase space-separated hex (e.g. `00 3E 05`) to stdout.
//!
//! `sigil parse <input.emp>` runs only the .emp lexer/parser front end
//! (Spec 2 Plan 1) and reports success or every diagnostic collected.
//!
//! `sigil build --aeon <dir>` is THE Aeon ROM build (post-flip, the only one):
//! it assembles the game's residual root `.asm` (`games/<game>/game_root.asm`)
//! with every `.emp` module lowered natively, chained-links, folds the checksum,
//! emits the sigil-canonical `.lst`, and appends the `convsym` deb2 symbol table —
//! the full shipped ROM. (It said `main.asm` until 2026-08-14; that file was
//! deleted at the flip this same sentence describes.)
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

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("--version") | Some("-V") => return run_version(),
        Some("parse") => return run_parse(),
        Some("emp") => return run_emp(&args[2..]),
        Some("test") => return run_test(&args[2..]),
        Some("build") => return run_build(&args[2..]),
        _ => {}
    }

    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut hex = false;

    let mut i = 1;
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
        None => {
            eprintln!("usage: sigil <input.asm> [-o <output.bin>] [--hex]");
            eprintln!("       sigil --version");
            process::exit(2);
        }
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
            render_as_warnings(&a);
            (a.module, a.sources)
        }
        Err(failure) => {
            render_as_diags(&failure);
            process::exit(1);
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
            process::exit(1);
        }
    };
    let linked = match sigil_link::link(&resolved, &empty) {
        Ok(img) => img,
        Err(diags) => {
            render_located_diags(&diags, &sources);
            process::exit(1);
        }
    };
    let image = sigil_link::flatten(&linked, 0x00);

    if let Some(out_path) = output {
        if let Err(err) = std::fs::write(&out_path, &image) {
            eprintln!("error: cannot write {out_path}: {err}");
            process::exit(1);
        }
    }

    if hex {
        let rendered: Vec<String> = image.iter().map(|b| format!("{b:02X}")).collect();
        println!("{}", rendered.join(" "));
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
fn run_version() {
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
fn run_parse() {
    let path = match std::env::args().nth(2) {
        Some(path) => path,
        None => {
            eprintln!("usage: sigil parse <file.emp>");
            process::exit(2);
        }
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
) -> Result<(sigil_link::LinkedImage, Vec<sigil_span::Diagnostic>), Vec<sigil_span::Diagnostic>> {
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
    Ok((image, assert_diags))
}

/// The no-map link seam: [`link_to_image`] then `flatten` (gap-fill 0x00, no
/// region validation).
fn link_sections(
    sections: &[sigil_ir::Section],
    asserts: &[sigil_ir::LinkAssert],
) -> Result<(Vec<u8>, Vec<sigil_span::Diagnostic>), Vec<sigil_span::Diagnostic>> {
    let (image, warns) = link_to_image(sections, asserts)?;
    Ok((sigil_link::flatten(&image, 0x00), warns))
}

/// The shared emp output tail: write `image` to `output` (if given), print it as
/// `--hex` (if set), and always report `built: N bytes`. Exits non-zero on a
/// write failure.
fn emit_image(image: &[u8], output: Option<&str>, hex: bool) {
    if let Some(out_path) = output {
        if let Err(err) = std::fs::write(out_path, image) {
            eprintln!("error: cannot write {out_path}: {err}");
            process::exit(1);
        }
    }
    if hex {
        let rendered: Vec<String> = image.iter().map(|b| format!("{b:02X}")).collect();
        println!("{}", rendered.join(" "));
    }
    println!("built: {} bytes", image.len());
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
fn run_test(args: &[String]) {
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
        (None, None) => {
            eprintln!("usage: sigil test <input.emp> [--root <dir>] [-D NAME=INT]...");
            process::exit(2);
        }
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
fn run_emp(args: &[String]) {
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
        None => {
            eprintln!(
                "usage: sigil emp <input.emp> [--root <dir>] [--prelude <module.id>] \
                 [-o <output.bin>] [--hex] [-D NAME=INT]..."
            );
            process::exit(2);
        }
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
    if map_arg.is_some() {
        eprintln!("error: --map requires --root (region placement is a multi-module concern)");
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

    emit_image(&image, output.as_deref(), hex);
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

    emit_image(&image, output, hex);
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
    let (linked, warns) = link_to_image(sections, asserts)?;
    sigil_link::emit_rom(&linked, map).map(|rom| (rom, warns)).map_err(|msg| {
        vec![sigil_span::Diagnostic {
            level: sigil_span::Level::Error,
            message: msg,
            // A region/placement failure belongs to no source line. An id past
            // every scanned file makes the renderer degrade to a bare
            // `error: <msg>` rather than attribute it to whichever module happens
            // to hold `SourceId(0)`.
            primary: sigil_span::Span { source: sigil_span::SourceId(u32::MAX), start: 0, end: 0 },
        }]
    })
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
fn run_build(args: &[String]) {
    let opts = match parse_build_args(args) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("error: {msg}");
            eprintln!(
                "usage: sigil build --aeon <dir> [-o <out.bin>] [--emit-lst <lst>] \
                 [--game sonic4|demo] [--debug] [--config-a|--config-b|--lean] \
                 [--report ram|contracts] [--extra-entry <module|path.emp>]...\n\
                 note:  --extra-entry evaluates the NAMED module's comptime guards; the \
                 named module must emit nothing (its own imports are not checked)\n\
                 env:   SIGIL_WARNINGS=off|summary|full  (warn-tier detail; default summary)"
            );
            process::exit(2);
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
}

impl ReportKind {
    /// Parse the `--report` value. Unrecognised is a usage error, never a silent
    /// fallback: a typo must not print the wrong report.
    fn parse(value: &str) -> Result<ReportKind, String> {
        match value {
            "ram" => Ok(ReportKind::Ram),
            "contracts" => Ok(ReportKind::Contracts),
            other => Err(format!("unknown --report '{other}' (want ram or contracts)")),
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
    Ok(BuildOpts { aeon, output, emit_lst, target, report, extra_entries })
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

    // The sigil-canonical listing (the `.lst`-consumer drop-in), if requested.
    if let Some(lst_path) = &opts.emit_lst {
        if let Err(err) = std::fs::write(lst_path, sigil_link::emit_listing(&listing)) {
            eprintln!("error: cannot write {lst_path}: {err}");
            process::exit(1);
        }
    }

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
    if let Some(out_path) = &opts.output {
        if let Err(err) = std::fs::write(out_path, &full) {
            eprintln!("error: cannot write {out_path}: {err}");
            process::exit(1);
        }
    }
    println!("built: {label} native ROM, crc={:08x} len={}", native::crc32(&full), full.len());
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
