//! Every stdout write in a sigil binary goes through `sigil_harness::stdout`, so
//! the broken-pipe rule that module states holds for every binary, not only for
//! the ones someone remembered.
//!
//! # The population, by the property rather than by a list
//!
//! A stdout write is a `print!` or `println!` call, or a stdout handle taken with
//! `stdout()`. The scan reads every `.rs` file under each crate's `src/`:
//!
//! - a BINARY ROOT (`src/main.rs`, `src/bin/*.rs`, and every `[[bin]]` path a
//!   manifest names) that calls `print!` must import `print` from
//!   `sigil_harness::stdout`, and one that calls `println!` must import `println`,
//!   because that import is what makes the call the rule's rather than std's;
//! - no file may take a stdout handle, which writes around the macros, except the
//!   rule's own module;
//! - no LIBRARY file may print to stdout at all: its `println!` is std's in every
//!   binary that calls it, and no import in the binary reaches it.
//!
//! Only the shipping text of a file is read. Each top-level inline
//! `#[cfg(test)] mod <name> {` block is removed, from its attribute to the
//! column-zero `}` that closes it, because a test's output goes to libtest and no
//! reader of it goes away; code after the block is read. A comment line is
//! skipped.
//!
//! # Outside the population
//!
//! - `build.rs`: cargo reads a build script's stdout to the end.
//! - `tests/` and `examples/`: libtest, and a developer running an example.
//! - `eprint!` and `eprintln!`: the rule covers stdout only.
//!
//! # Controls
//!
//! The detector is run on snippets whose answers are known, and the scan must
//! find `sigil`'s own `main.rs` among the writers, since `sigil --version` is
//! written there. Without them, a scan that matched nothing would pass.

use std::path::{Path, PathBuf};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

/// Every `.rs` file under `dir`, recursively, in a stable order.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// The binary roots of the crate at `crate_dir`: the paths cargo discovers a
/// binary at, and every `path` under a `[[bin]]` table of its manifest.
fn bin_roots(crate_dir: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let main = crate_dir.join("src/main.rs");
    if main.is_file() {
        roots.push(main);
    }
    let bin_dir = crate_dir.join("src/bin");
    if bin_dir.is_dir() {
        for e in std::fs::read_dir(&bin_dir).expect("read src/bin") {
            let p = e.expect("dir entry").path();
            if p.extension().is_some_and(|x| x == "rs") {
                roots.push(p);
            }
        }
    }
    let manifest = std::fs::read_to_string(crate_dir.join("Cargo.toml")).expect("read Cargo.toml");
    let mut in_bin = false;
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_bin = t == "[[bin]]";
        } else if in_bin {
            if let Some(v) = t.strip_prefix("path").map(str::trim_start).and_then(|r| r.strip_prefix('=')) {
                roots.push(crate_dir.join(v.trim().trim_matches('"')));
            }
        }
    }
    roots
}

/// The shipping text of a source file: the file with each top-level inline
/// `#[cfg(test)] mod <name> {` block removed, from its attribute to the first
/// line holding only `}`, which is where rustfmt closes a module at column zero.
/// Code after the block is kept. A `#[cfg(test)] mod <name>;` naming a file is
/// not a block, and an indented attribute is not top-level; both are kept, so a
/// print under either is read as shipping, which fails loudly rather than
/// passing quietly.
fn shipping(src: &str) -> String {
    const ATTR: &str = "#[cfg(test)]";
    let mut out = String::new();
    let mut kept_from = 0;
    let mut search = 0;
    while let Some(rel) = src[search..].find(ATTR) {
        let at = search + rel;
        let top_level = at == 0 || src.as_bytes()[at - 1] == b'\n';
        let first = src[at + ATTR.len()..].trim_start().lines().next().unwrap_or("");
        if top_level && first.starts_with("mod ") && first.trim_end().ends_with('{') {
            out.push_str(&src[kept_from..at]);
            kept_from = src[at..].find("\n}\n").map_or(src.len(), |end| at + end + 3);
            search = kept_from;
        } else {
            search = at + ATTR.len();
        }
    }
    out.push_str(&src[kept_from..]);
    out
}

/// The stdout writes a text makes.
#[derive(Default, Debug)]
struct Writes {
    print: bool,
    println: bool,
    handle: bool,
}

/// Whether `line` invokes the macro `name` (`print!` or `println!`) itself,
/// rather than a longer name ending in it such as `eprintln!`.
fn invokes(line: &str, name: &str) -> bool {
    let mut from = 0;
    while let Some(rel) = line[from..].find(name) {
        let at = from + rel;
        let own_name = line[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let opens = line[at + name.len()..].starts_with(['(', '{', '[']);
        if own_name && opens {
            return true;
        }
        from = at + name.len();
    }
    false
}

/// The stdout writes in `text`, comment lines skipped.
fn writes(text: &str) -> Writes {
    let mut w = Writes::default();
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        w.print |= invokes(t, "print!");
        w.println |= invokes(t, "println!");
        w.handle |= t.contains("stdout()");
    }
    w
}

/// The names `text` imports from `sigil_harness::stdout`.
fn imported(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("use sigil_harness::stdout::") else {
            continue;
        };
        let list = rest.trim_end_matches(';').trim_matches(|c| c == '{' || c == '}');
        names.extend(list.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
    }
    names
}

#[test]
fn every_stdout_write_in_a_binary_goes_through_the_broken_pipe_rule() {
    let ws = workspace();
    let helper = ws.join("crates/sigil-harness/src/stdout.rs");
    let mut crates: Vec<PathBuf> = std::fs::read_dir(ws.join("crates"))
        .expect("read crates/")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.join("Cargo.toml").is_file())
        .collect();
    crates.sort();

    let mut scanned = 0usize;
    let mut writers: Vec<String> = Vec::new();
    let mut violations: Vec<String> = Vec::new();
    for crate_dir in &crates {
        let src_dir = crate_dir.join("src");
        if !src_dir.is_dir() {
            continue;
        }
        let bins = bin_roots(crate_dir);
        let mut files = Vec::new();
        rust_files(&src_dir, &mut files);
        for file in files {
            if file == helper {
                continue;
            }
            scanned += 1;
            let rel = file.strip_prefix(&ws).unwrap_or(&file).display().to_string();
            let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {rel}: {e}"));
            let ship = shipping(&text);
            let w = writes(&ship);
            if w.handle {
                violations.push(format!(
                    "{rel}: takes a stdout handle, which writes around `sigil_harness::stdout`"
                ));
            }
            if !(w.print || w.println) {
                continue;
            }
            if !bins.contains(&file) {
                violations.push(format!(
                    "{rel}: a library file printing to stdout. That `println!` is std's in every \
                     binary that calls it and panics on a closed stdout; return the text and let \
                     the binary print it"
                ));
                continue;
            }
            writers.push(rel.clone());
            let names = imported(&ship);
            for (used, name) in [(w.print, "print"), (w.println, "println")] {
                if used && !names.iter().any(|n| n == name) {
                    violations.push(format!(
                        "{rel}: calls `{name}!` without `use sigil_harness::stdout::{name}`, so its \
                         writes are std's and panic on a closed stdout"
                    ));
                }
            }
        }
    }

    assert!(
        writers.iter().any(|w| w == "crates/sigil-cli/src/main.rs"),
        "the scan did not find crates/sigil-cli/src/main.rs among the stdout writers, which \
         prints `sigil --version`; it is not reading what it claims to. Writers found: {writers:?}"
    );
    assert!(
        violations.is_empty(),
        "{} stdout write(s) outside the broken-pipe rule, over {scanned} file(s) and {} binary \
         writer(s):\n  {}",
        violations.len(),
        writers.len(),
        violations.join("\n  ")
    );
}

/// The detector, on snippets whose answers are known.
#[test]
fn the_detector_answers_known_snippets() {
    assert!(writes("    println!(\"x\");").println);
    assert!(writes("print!(\"{}\", s);").print);
    assert!(writes("        None => print!(\"{}\", top_level_help()),").print);
    assert!(!writes("    eprintln!(\"x\");").println, "eprintln! is stderr");
    assert!(!writes("    eprint!(\"x\");").print, "eprint! is stderr");
    assert!(!writes("    // println!(\"x\");").println, "a comment line");
    assert!(!writes("    println!(\"x\");").print, "println! is not print!");
    assert!(writes("let mut out = std::io::stdout().lock();").handle);
    assert!(!writes(".stdout(Stdio::piped())").handle, "configuring a child's stdout");

    assert_eq!(imported("use sigil_harness::stdout::{print, println};"), ["print", "println"]);
    assert_eq!(imported("use sigil_harness::stdout::println;"), ["println"]);
    assert!(imported("use sigil_harness::native;").is_empty());

    let src = "fn main() {}\n#[cfg(test)]\nmod named_file;\nfn kept() {}\n\
               #[cfg(test)]\nmod tests {\n    fn t() { println!(\"z\"); }\n}\n\
               fn after() { print!(\"y\"); }\n";
    let ship = shipping(src);
    assert!(ship.contains("fn kept()"), "a `mod x;` under cfg(test) is not a block");
    assert!(!writes(&ship).println, "an inline test module is removed");
    assert!(writes(&ship).print, "shipping code after a test module is read");

    let nested = "mod outer {\n    #[cfg(test)]\n    mod t {\n        fn x() {}\n    }\n\
                  \x20   fn y() { println!(\"n\"); }\n}\n";
    assert!(writes(&shipping(nested)).println, "an indented test module is not removed");
}
