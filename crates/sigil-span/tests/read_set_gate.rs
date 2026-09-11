//! THE READ-SET SOURCE GATE: no file read or tool spawn a `sigil build` can reach may
//! bypass `sigil_span::read_set`.
//!
//! The `.lst` source digest names every file the build read. It is complete because
//! every build read goes through the recorder, and this gate is what makes that a
//! property of the source rather than of the current build's code paths: a raw
//! `std::fs::read`, `std::fs::read_to_string`, `File::open`, an `OpenOptions` read or a
//! `Command::new` in shipping code is refused unless the site says why it is not a
//! build input. The runtime witness in `crates/sigil-cli/tests/lst_source_digest.rs`
//! checks the same property from the other side, over real builds, against the
//! kernel's own record of what was opened.
//!
//! # What it scans
//!
//! Every `.rs` file under `crates/*/src/`, cut at its first `#[cfg(test)]` module,
//! except:
//!  * `src/bin/` trees: separate executables that `sigil build` never runs (the library
//!    code they call is scanned where it lives);
//!  * `crates/sigil-span/src/read_set.rs`: the recorder itself, whose raw reads are the
//!    ones every other site routes through;
//!  * `crates/sigil-cli/src/main.rs`, which hosts every subcommand: there the gate scans
//!    the functions reachable from `run_build` by the file's own call graph, so a
//!    `sigil emp` input read is out of scope and a new helper on the build path is not.
//!
//! This file lives under `tests/`, outside that scope, so the needles spelled below
//! neither satisfy nor trip it.
//!
//! # The exemption
//!
//! A line carrying `read-set: not a build input`, or with it in one of the three lines
//! above, is exempt, and so is every line of a file whose leading `//!` module doc
//! carries it. The marker has to state its reason in prose beside it;
//! the gate cannot judge the reason, only require that one was written where a reviewer
//! will see it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MARKER: &str = "read-set: not a build input";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("workspace root")
}

/// Each raw file-read or spawn spelling on `line`, by the name the report uses.
fn raw_reads(line: &str) -> Vec<&'static str> {
    let code = line.split("//").next().unwrap_or("");
    let mut found = Vec::new();
    let mut rest = code;
    while let Some(i) = rest.find("fs::read") {
        let after = &rest[i + "fs::read".len()..];
        if !after.starts_with("_dir") && !after.starts_with("_link") {
            found.push("std::fs::read / read_to_string");
        }
        rest = after;
    }
    if code.contains("File::open(") {
        found.push("File::open");
    }
    if code.contains(".read(true)") {
        found.push("OpenOptions read");
    }
    if code.contains("Command::new(") {
        found.push("Command::new");
    }
    if code.trim_start().starts_with("use std::fs::{") && code.contains("read") {
        found.push("a std::fs read import");
    }
    found
}

/// The shipping text of a source file: everything before its first INLINE test module
/// (`#[cfg(test)]` then `mod name {`). An out-of-line `#[cfg(test)] mod name;` cuts
/// nothing here; [`test_module_files`] drops the file it names instead.
fn shipping(text: &str) -> &str {
    let mut offset = 0;
    let mut prev_was_cfg_test = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        let inline_mod = (t.starts_with("mod ") || t.starts_with("pub mod ")) && t.ends_with('{');
        if prev_was_cfg_test && inline_mod {
            let cfg = text[..offset].rfind("#[cfg(test)]").expect("the cfg line precedes");
            return &text[..cfg];
        }
        if !t.is_empty() {
            prev_was_cfg_test = t == "#[cfg(test)]";
        }
        offset += line.len();
    }
    text
}

/// The files `path` declares as out-of-line test modules (`#[cfg(test)]` then
/// `mod name;`), resolved the way rustc resolves them: beside a `main.rs`, `lib.rs` or
/// `mod.rs`, and in a directory named after any other file's stem.
fn test_module_files(path: &Path, text: &str) -> Vec<PathBuf> {
    let dir = match path.file_name().and_then(|n| n.to_str()) {
        Some("main.rs" | "lib.rs" | "mod.rs") => path.parent().map(Path::to_path_buf),
        _ => path.parent().zip(path.file_stem()).map(|(p, s)| p.join(s)),
    };
    let Some(dir) = dir else { return Vec::new() };
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let mut out = Vec::new();
    for pair in lines.windows(2) {
        let decl = pair[1].strip_prefix("pub ").unwrap_or(pair[1]);
        if pair[0] == "#[cfg(test)]" {
            if let Some(name) = decl.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
                out.push(dir.join(format!("{name}.rs")));
                out.push(dir.join(name).join("mod.rs"));
            }
        }
    }
    out
}

/// A file exempts itself with the marker in its leading `//!` module doc.
fn file_is_exempt(text: &str) -> bool {
    text.lines().take_while(|l| l.starts_with("//!")).any(|l| l.contains(MARKER))
}

/// `(line number, needle)` for every unexempted raw read in `text`, numbered from
/// `first_line`.
fn violations(text: &str, first_line: usize) -> Vec<(usize, &'static str)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        for needle in raw_reads(line) {
            let lo = i.saturating_sub(3);
            if !lines[lo..=i].iter().any(|l| l.contains(MARKER)) {
                out.push((first_line + i, needle));
            }
        }
    }
    out
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "bin") {
                continue;
            }
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Every top-level `fn` in `src` with its body: from `fn <name>(` at column 0 (or after
/// `pub `) to the next `}` at column 0.
fn top_level_fns(src: &str) -> BTreeMap<String, (usize, &str)> {
    let mut fns = BTreeMap::new();
    let mut offset = 0;
    for (line_no, line) in (1..).zip(src.split_inclusive('\n')) {
        let head = line.strip_prefix("pub ").unwrap_or(line);
        if let Some(sig) = head.strip_prefix("fn ") {
            let name: String = sig.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            let body_from = offset;
            let rest = &src[body_from..];
            let end = rest.find("\n}\n").map(|i| i + 3).unwrap_or(rest.len());
            fns.insert(name, (line_no, &src[body_from..body_from + end]));
        }
        offset += line.len();
    }
    fns
}

/// The names of `fns` reachable from `root` through calls spelled `name(` with no
/// identifier character or `.` / `::` before them.
fn reachable(fns: &BTreeMap<String, (usize, &str)>, root: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut work = vec![root.to_string()];
    while let Some(name) = work.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let Some((_, body)) = fns.get(&name) else { continue };
        for callee in fns.keys() {
            let needle = format!("{callee}(");
            let mut rest = *body;
            while let Some(i) = rest.find(&needle) {
                let before = rest[..i].chars().last();
                let bare = before.is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '.' || c == ':'));
                if bare && !seen.contains(callee) {
                    work.push(callee.clone());
                }
                rest = &rest[i + needle.len()..];
            }
        }
    }
    seen
}

#[test]
fn the_matcher_flags_raw_reads_and_passes_the_recorder() {
    for flagged in [
        "    let b = std::fs::read(&p)?;",
        "    let s = std::fs::read_to_string(p).unwrap();",
        "    let f = std::fs::File::open(p)?;",
        "    let o = OpenOptions::new().read(true).open(p);",
        "    let out = std::process::Command::new(&convsym)",
        "use std::fs::{self, read};",
        "    let s = fs::read_to_string(p)?;",
        "    let v: Vec<_> = paths.iter().map(std::fs::read).collect();",
    ] {
        assert!(!raw_reads(flagged).is_empty(), "not flagged: {flagged}");
        assert_eq!(violations(flagged, 1).len(), 1, "an unmarked raw read must be a violation: {flagged}");
    }
    for passed in [
        "    let b = sigil_span::read_set::read(&p)?;",
        "    let s = sigil_span::read_set::read_to_string(p)?;",
        "    for e in std::fs::read_dir(dir)? {",
        "    let t = std::fs::read_link(p)?;",
        "    // std::fs::read(&p) in a comment says nothing",
        "    let mut cmd = sigil_span::read_set::tool_command(&convsym)?;",
    ] {
        assert!(raw_reads(passed).is_empty(), "flagged: {passed}");
    }
    let marked = format!("    // {MARKER}: the build's own output, read back\n    let b = std::fs::read(&p)?;\n");
    assert!(violations(&marked, 1).is_empty(), "a marked site must pass");
    let far = format!("    // {MARKER}\n\n\n\n    let b = std::fs::read(&p)?;\n");
    assert_eq!(violations(&far, 1).len(), 1, "a marker four lines up must not reach");
}

#[test]
fn the_shipping_cut_keeps_code_and_drops_the_test_module() {
    let text = "fn a() {}\n#[cfg(test)]\nmod tests {\n    fn t() { std::fs::read(\"x\"); }\n}\n";
    assert_eq!(shipping(text), "fn a() {}\n");
    let gated_item = "#[cfg(test)]\nfn helper() {}\nfn b() { std::fs::read(\"x\"); }\n";
    assert_eq!(shipping(gated_item), gated_item, "a cfg(test) item that is not a module cuts nothing");
    let out_of_line = "#[cfg(test)]\nmod tree_class;\nfn c() { std::fs::read(\"x\"); }\n";
    assert_eq!(shipping(out_of_line), out_of_line, "an out-of-line test module cuts nothing here");
    assert_eq!(
        test_module_files(Path::new("/w/crates/x/src/main.rs"), out_of_line),
        vec![PathBuf::from("/w/crates/x/src/tree_class.rs"), PathBuf::from("/w/crates/x/src/tree_class/mod.rs")]
    );
    assert_eq!(
        test_module_files(Path::new("/w/crates/x/src/eval.rs"), out_of_line)[0],
        PathBuf::from("/w/crates/x/src/eval/tree_class.rs")
    );
}

#[test]
fn no_build_reachable_read_bypasses_the_recorder() {
    let ws = workspace();
    let recorder = ws.join("crates/sigil-span/src/read_set.rs");
    let cli_main = ws.join("crates/sigil-cli/src/main.rs");

    let mut files = Vec::new();
    for krate in std::fs::read_dir(ws.join("crates")).expect("crates dir").flatten() {
        rust_files(&krate.path().join("src"), &mut files);
    }
    files.sort();

    let test_only: BTreeSet<PathBuf> = files
        .iter()
        .flat_map(|p| test_module_files(p, &std::fs::read_to_string(p).expect("read a source file")))
        .collect();

    let mut report = Vec::new();
    let mut scanned: BTreeSet<PathBuf> = BTreeSet::new();
    let mut calls_recorder: BTreeSet<PathBuf> = BTreeSet::new();
    for path in &files {
        if *path == recorder || *path == cli_main || test_only.contains(path) {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("read a source file");
        let text = shipping(&text);
        if text.contains("read_set::") {
            calls_recorder.insert(path.clone());
        }
        if file_is_exempt(text) {
            continue;
        }
        scanned.insert(path.clone());
        for (line, needle) in violations(text, 1) {
            report.push(format!("{}:{line}: {needle}", path.strip_prefix(&ws).unwrap_or(path).display()));
        }
    }

    // main.rs: the functions reachable from `run_build`.
    let main_text = std::fs::read_to_string(&cli_main).expect("read main.rs");
    let main_ship = shipping(&main_text);
    let fns = top_level_fns(main_ship);
    assert!(fns.contains_key("run_build"), "no top-level `fn run_build` in main.rs's shipping text");
    let closure = reachable(&fns, "run_build");
    for name in &closure {
        let (line, body) = fns[name];
        for (at, needle) in violations(body, line) {
            report.push(format!("crates/sigil-cli/src/main.rs:{at}: {needle} (in `{name}`, reachable from run_build)"));
        }
    }

    // The instrument, checked against what the source says it must have seen: the
    // native build path is in main.rs's closure, and every file that calls the recorder
    // was scanned rather than exempted.
    for must in ["run_build_native", "run_contract_gate", "lst_source_digest"] {
        assert!(closure.contains(must), "`{must}` is not reachable from run_build, the call graph is broken: {closure:?}");
    }
    assert!(calls_recorder.len() >= 6, "only {} files call the recorder, the scan found almost nothing: {calls_recorder:?}", calls_recorder.len());
    let unscanned: Vec<_> = calls_recorder.difference(&scanned).collect();
    assert!(unscanned.is_empty(), "files that call the recorder were exempted from the gate: {unscanned:?}");
    let recorder_text = std::fs::read_to_string(&recorder).expect("read the recorder");
    assert!(
        !violations(shipping(&recorder_text), 1).is_empty(),
        "the recorder holds no raw read, so its exclusion is not the thing keeping this gate green"
    );

    assert!(
        report.is_empty(),
        "{} raw read(s) or spawn(s) a sigil build can reach bypass sigil_span::read_set. Route \
         each through read_set::read / read_to_string / write_generated / tool_command, or, if it \
         is not a build input, say so with `{MARKER}` and the reason:\n  {}",
        report.len(),
        report.join("\n  ")
    );
}
