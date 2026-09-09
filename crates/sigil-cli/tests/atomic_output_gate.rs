//! Every artifact this binary hands to a consumer is installed by rename, and no
//! source file in the binary's crate reaches for a truncating whole-file write.
//!
//! # Why a source-text rule and not a behavioural one
//!
//! The property is about a RACE: a reader must never observe a partly-written
//! artifact. A test that writes a file and then reads it does not exercise that race,
//! and a test that tries to (a reader thread spinning on the path) is timing-shaped
//! and would be a flaky gate, which is worse than an honest gap. So the race itself
//! is unproven here, on purpose, and what IS gated is the thing that actually
//! regresses: a future output site written the old way. The atomic installer's own
//! behaviour is proven directly by the unit tests beside it.
//!
//! # Why the rule can be absolute
//!
//! The banned call has no legitimate use in this crate's sources, because the
//! installer is a strict superset of it: it takes the same path and the same bytes,
//! and it installs them by rename instead of by truncation. A check that fires on
//! correct code trains people to weaken it; this one cannot, because there is no
//! correct code it fires on. A fixture written by an inline unit test is served by
//! the installer as well as by the banned call.
//!
//! # The scope this states, and the name it does not
//!
//! The population is a DIRECTORY, walked at test time, not a list of files or
//! functions written down here. A source-text check whose doc names a live member of
//! the population it scans rots the moment that member moves or is renamed, and a
//! reader then trusts a stale list over the walk. So this file names the shape (a
//! whole-file truncating write, anywhere under the crate's sources) and never an
//! individual site, symbol, or line.
//!
//! This test file is not itself under the scanned directory, so the scan does not
//! read its own source and cannot count its own prose as a member.

use std::path::{Path, PathBuf};

/// The truncating whole-file write this crate's sources must not contain. Spelled in
/// pieces so this file's own mention of it is not the literal the scanner looks for,
/// which would otherwise make the gate unable to describe its own subject.
const BANNED: &str = concat!("fs", "::", "write", "(");

/// Every `.rs` file under `dir`, recursively.
fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).unwrap_or_else(|e| panic!("read_dir {}: {e}", d.display()))
        {
            let entry = entry.expect("dir entry");
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Lines of `text` containing the banned call, as `(1-based line, trimmed text)`.
fn offending_lines(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| l.contains(BANNED))
        .map(|(i, l)| (i + 1, l.trim().to_string()))
        .collect()
}

/// POSITIVE CONTROL for the matcher, run in the same process as the gate below.
///
/// An emptiness is only a finding if the instrument that produced it could have
/// produced something else. This proves the predicate fires on the shape it is
/// looking for and stays quiet on the installer that replaces it, so the gate's zero
/// is a statement about the sources rather than about a predicate that never matches.
#[test]
fn the_scanner_finds_the_shape_it_bans() {
    let planted = "fn f() {\n    std::fs::write(&p, &bytes).unwrap();\n}\n";
    assert_eq!(
        offending_lines(planted).len(),
        1,
        "the predicate did not fire on a planted truncating write"
    );
    let replacement = "fn f() {\n    install_artifact(&p, &bytes).unwrap();\n}\n";
    assert!(
        offending_lines(replacement).is_empty(),
        "the predicate fires on the installer that is supposed to replace the banned call"
    );
}

/// THE GATE. No source file in this crate performs a truncating whole-file write.
#[test]
fn no_source_in_this_crate_writes_a_file_by_truncation() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = rust_sources(&src);

    // INPUT ASSERTION. A walk that reached nothing would report a clean crate, and a
    // clean report from an empty walk is indistinguishable from a clean report from a
    // clean crate. The floor is derived from what a binary crate must have at minimum
    // (its own root) plus a margin, not copied from a current count, so splitting or
    // merging a module does not move it.
    assert!(
        files.len() >= 2,
        "scanned only {} source file(s) under {} - the walk found nothing to check",
        files.len(),
        src.display()
    );
    assert!(
        files.iter().any(|p| p.file_name().is_some_and(|n| n == "main.rs")),
        "the walk did not reach the binary's own root under {}",
        src.display()
    );

    let mut findings = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap_or_else(|e| panic!("read {}: {e}", f.display()));
        for (line, body) in offending_lines(&text) {
            findings.push(format!("{}:{line}: {body}", f.display()));
        }
    }

    assert!(
        findings.is_empty(),
        "a truncating whole-file write in this crate's sources leaves a window in which \
         the artifact exists and is empty or partial, which a consumer polling it during \
         a build reads as a complete short file.\n\
         Install the bytes by rename instead - see sigil_harness::atomic_write, reached \
         from this crate through the one output-installing helper in main.rs.\n\
         Scanned {} file(s); offending:\n  {}",
        files.len(),
        findings.join("\n  ")
    );
}
