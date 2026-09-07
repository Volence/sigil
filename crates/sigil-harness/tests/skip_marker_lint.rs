//! THE ZERO-SKIP BAR CANNOT SEE A SPELLING NOBODY GUESSED.
//!
//! sigil's landing bar and `scripts/nightly_source_gates.sh` both fail a strict
//! suite run that emits a skip line, because a skip line is a gate that measured
//! nothing while reporting green. Both enforcers match on a STRING. That makes
//! the bar exactly as wide as the spellings whoever wrote it happened to guess,
//! and no wider: 29 announced early returns across ten test files said
//! `skipping <gate> …` and `skip <gate> …`, neither of which matches `skip: `,
//! so those gates could no-op and clear the bar while reading back as coverage.
//!
//! Re-spelling them closed the sites that existed. This closes the ones that do
//! not exist yet: a NEW announced early return written in an unmatched spelling
//! fails here, in the same workspace run that enforces the bar.
//!
//! WHAT IT ENUMERATES OVER — the property, not the literal. Enumerating by the
//! string `skipping` is the same mistake that made the hole: it finds only what
//! you already guessed. Two independent detectors run, and their UNION must
//! carry [`SKIP_MARKER`]:
//!
//!   A. STRUCTURAL — a print macro with an early `return` within the next few
//!      lines. Catches a phrasing with no skip vocabulary at all
//!      (`eprintln!("no tree here"); return;`).
//!   B. LEXICAL — a print macro whose text uses skip vocabulary. Catches an
//!      announcement whose exit is a `let … else`, a `?`, or sits further down
//!      the function than the structural window reaches — 31 of the corpus's
//!      current announcement sites are visible ONLY to this detector.
//!
//! Neither detector alone covers the other's set, which is why both run.
//!
//! WHAT IT CANNOT CATCH — stated because an assertion of completeness and the
//! check that would establish it are separable, and only the assertion is cheap:
//!
//!   * A SILENT early return. `if !something() { return; }` with no print at all
//!     announces nothing, so no marker discipline can reach it — and it is worse
//!     than a mis-spelled skip, because it is invisible to every log grep.
//!   * An announcement built at runtime — `eprintln!("{}", msg)` where `msg` is
//!     assembled elsewhere. The scanner reads the format literal in the source
//!     and cannot follow a value.
//!   * A gate that runs to completion but asserts nothing (a vacuous body). That
//!     is a different defect class with a different bar (red-first proof).
//!   * Announcements outside this scope: `crates/*/src/**` other than
//!     `test_support.rs`, where the bin targets legitimately print-and-return on
//!     error paths that are not gate skips.
//!
//! Those residuals are ledgered in `docs/superpowers/notes/campaign-gap-ledger.md`.

use sigil_harness::test_support::SKIP_MARKER;
use std::path::{Path, PathBuf};

/// How many lines after a print macro count as "this print announces that exit".
/// Sized off the corpus's own guard idiom, which is at most a print, a comment
/// line and the `return` — widened by two so a reformat does not silently narrow
/// the detector.
const STRUCTURAL_WINDOW: usize = 5;

/// The vocabulary detector B recognises. Deliberately broader than the marker:
/// its job is to notice a site that is ABOUT skipping without having been
/// spelled as one.
const SKIP_VOCABULARY: &[&str] = &[
    "skip",
    "not measured",
    "unavailable",
    "absent",
    "missing",
    "not found",
    "no reference",
];

fn crates_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is <workspace>/crates/sigil-harness.
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .unwrap_or_else(|| panic!("no parent of {}", here.display()))
        .to_path_buf()
}

/// Every file whose print macros can land in a strict suite log as a gate
/// announcement: the whole test tree, plus the shared guard helper that
/// announces on the tests' behalf.
fn scanned_files() -> Vec<PathBuf> {
    let crates = crates_dir();
    assert!(
        crates.is_dir(),
        "COULD NOT MEASURE: the crates tree is not at {}, this lint scanned nothing, \
         which is not the same as finding nothing",
        crates.display()
    );

    let mut out = Vec::new();
    let mut crate_dirs: Vec<PathBuf> = std::fs::read_dir(&crates)
        .unwrap_or_else(|e| panic!("read {}: {e}", crates.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    crate_dirs.sort();

    for c in crate_dirs {
        collect_rs(&c.join("tests"), &mut out);
    }
    let helper = crates.join("sigil-harness/src/test_support.rs");
    if helper.is_file() {
        out.push(helper);
    }
    // This file's own doc comment quotes the very spellings it forbids.
    let me = Path::new(file!())
        .file_name()
        .map(|n| n.to_owned())
        .expect("this file has a name");
    out.retain(|p| p.file_name() != Some(me.as_os_str()));
    out.sort();
    out
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// The format literal of the first `eprintln!`/`println!` whose invocation begins
/// on the line of `src` starting at byte `line_start`, when the macro's first
/// argument is a string literal. THE LITERAL IS READ FROM `src`, NOT FROM THE
/// LINE: rustfmt puts a long literal on the line after `println!(`, and a
/// literal that does not fit one line continues past a `\` at the line end.
/// A detector that stopped at the line break saw neither, and five live
/// announcements in the lint's own forbidden vocabulary were green for exactly
/// that reason. Returns `None` for a print whose first argument is a value; see
/// the residuals in the module doc.
fn print_literal(src: &str, line_start: usize) -> Option<String> {
    let line_end = src[line_start..]
        .find('\n')
        .map_or(src.len(), |n| line_start + n);
    let line = &src[line_start..line_end];
    for macro_name in ["eprintln!(", "println!("] {
        let mut from = 0usize;
        while let Some(rel) = line[from..].find(macro_name) {
            let at = from + rel;
            // `eprintln!` also ends in `println!`; take the longest match by
            // checking the character before the name is not an identifier char.
            let preceded_ok = line[..at]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_' && c != 'e');
            from = at + macro_name.len();
            if !preceded_ok {
                continue;
            }
            // From the open paren to the end of the FILE: whitespace before the
            // literal may include any number of line breaks.
            let rest = src[line_start + at + macro_name.len()..].trim_start();
            if !rest.starts_with('"') {
                continue;
            }
            return Some(read_string_literal(rest));
        }
    }
    None
}

/// The body of the string literal `rest` opens with (`rest[0] == '"'`), up to
/// its closing quote. A `\` at a line end is Rust's continuation: the break and
/// the next line's leading whitespace are dropped, as the compiler drops them,
/// so the vocabulary check sees the words the program prints. Every other
/// escape is kept verbatim; the marker check reads only the prefix, and a
/// literal cannot start with `\`.
fn read_string_literal(rest: &str) -> String {
    let bytes = rest.as_bytes();
    let mut i = 1usize;
    let mut lit = String::new();
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if matches!(bytes.get(i + 1), Some(b'\n' | b'\r')) => {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
            b'\\' => {
                lit.push('\\');
                if i + 1 < bytes.len() {
                    lit.push(bytes[i + 1] as char);
                }
                i += 2;
            }
            b'"' => return lit,
            b => {
                lit.push(b as char);
                i += 1;
            }
        }
    }
    lit
}

/// `return` as a token, not as a substring of `returned`/`_return`.
fn has_return_keyword(line: &str) -> bool {
    let b = line.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = line[from..].find("return") {
        let at = from + rel;
        let before_ok = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
        let end = at + "return".len();
        let after_ok = end >= b.len() || !(b[end].is_ascii_alphanumeric() || b[end] == b'_');
        if before_ok && after_ok {
            return true;
        }
        from = at + 1;
    }
    false
}

fn uses_skip_vocabulary(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    SKIP_VOCABULARY.iter().any(|w| lower.contains(w))
}

struct Site {
    file: PathBuf,
    line: usize,
    text: String,
    structural: bool,
    lexical: bool,
}

fn announcement_sites() -> (Vec<Site>, usize) {
    let files = scanned_files();
    let mut sites = Vec::new();
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            panic!(
                "COULD NOT MEASURE: {} is in scope but unreadable",
                path.display()
            );
        };
        sites.extend(scan_source(&src, path));
    }
    (sites, files.len())
}

/// Both detectors over ONE source text. Separated from the file walk so the
/// detector can be run over a fixture string, which is how its own blind spots
/// are tested below.
fn scan_source(src: &str, path: &Path) -> Vec<Site> {
    let lines: Vec<&str> = src.lines().collect();
    // Byte offset of each line's start, so the detector can read past the line
    // it was found on. `split_inclusive` keeps every line's true length (a
    // `\r\n` file would otherwise drift one byte per line).
    let mut starts = Vec::with_capacity(lines.len());
    let mut at = 0usize;
    for l in src.split_inclusive('\n') {
        starts.push(at);
        at += l.len();
    }
    let mut sites = Vec::new();
    for (i, start) in starts.iter().enumerate() {
        let Some(text) = print_literal(src, *start) else {
            continue;
        };
        let structural = lines
            .iter()
            .skip(i)
            .take(STRUCTURAL_WINDOW)
            .any(|l| has_return_keyword(l));
        let lexical = uses_skip_vocabulary(&text);
        if structural || lexical {
            sites.push(Site {
                file: path.to_path_buf(),
                line: i + 1,
                text,
                structural,
                lexical,
            });
        }
    }
    sites
}

// ---- the detector's own poison ---------------------------------------------
//
// Each fixture below is a shape the corpus contains, or contained when it slipped
// through. The vocabulary words in them are the lint's OWN forbidden words, which
// is the point: a fixture the detector cannot read is a hole every real site can
// use. This file is excluded from the walk (see `scanned_files`), so quoting them
// here does not count them.

/// rustfmt's wrapping: the literal starts on the line AFTER `println!(`. A
/// line-bound detector returned `None` here and the site was never a site.
#[test]
fn a_literal_that_starts_on_the_next_line_is_read() {
    let src = "fn f() {\n    println!(\n        \"NOT MEASURED: could not build the bed\"\n    );\n    return;\n}\n";
    let sites = scan_source(src, Path::new("fixture.rs"));
    assert_eq!(sites.len(), 1, "one announcement expected, got {}", sites.len());
    assert_eq!(sites[0].line, 2, "the site is reported at the macro's line");
    assert_eq!(sites[0].text, "NOT MEASURED: could not build the bed");
    assert!(sites[0].lexical, "`not measured` is skip vocabulary");
    assert!(sites[0].structural, "the `return` is inside the structural window");
    assert!(!sites[0].text.starts_with(SKIP_MARKER), "this fixture is a violator by construction");
}

/// A literal continued past a `\` at the line end: the compiler drops the break
/// and the next line's indentation, and so must the detector, or the words
/// either side of the break are never seen together.
#[test]
fn a_continued_literal_is_read_whole() {
    let src = "eprintln!(\n    \"NOT \\\n     MEASURED (secondary only): no ancestor\"\n);\n";
    let sites = scan_source(src, Path::new("fixture.rs"));
    assert_eq!(sites.len(), 1, "{:?}", sites.iter().map(|s| &s.text).collect::<Vec<_>>());
    assert_eq!(sites[0].text, "NOT MEASURED (secondary only): no ancestor");
    assert!(sites[0].lexical);
}

/// The compliant shapes stay compliant: a wrapped literal that carries the
/// marker is a site (it is counted in the census) and it passes the prefix test.
#[test]
fn a_wrapped_literal_carrying_the_marker_is_a_compliant_site() {
    let src = format!(
        "    eprintln!(\n        \"{SKIP_MARKER}reference ROM not at {{}} (set AEON_DIR)\",\n        p.display()\n    );\n    return None;\n"
    );
    let sites = scan_source(&src, Path::new("fixture.rs"));
    assert_eq!(sites.len(), 1);
    assert!(sites[0].text.starts_with(SKIP_MARKER), "{:?}", sites[0].text);
}

/// The single-line shape the detector always read, and the value-argument shape
/// it cannot read (a documented residual), so the widening changed neither.
#[test]
fn single_line_and_value_argument_shapes_are_unchanged() {
    let src = "eprintln!(\"skip: gate x, no tree\");\nreturn;\n";
    assert_eq!(print_literal(src, 0).as_deref(), Some("skip: gate x, no tree"));
    let src = "eprintln!(\"{}\", msg);\nreturn;\n";
    assert_eq!(print_literal(src, 0).as_deref(), Some("{}"));
    let src = "eprintln!(msg);\nreturn;\n";
    assert_eq!(print_literal(src, 0), None, "a value argument is not a literal");
}

/// THE GATE. Every announced early return in the test tree carries the one
/// spelling both enforcers of the zero-skip bar match on.
#[test]
fn every_announced_early_return_carries_the_skip_marker() {
    assert!(
        !SKIP_MARKER.is_empty(),
        "SKIP_MARKER is empty, every string starts with it and this lint is vacuous"
    );

    let (sites, files) = announcement_sites();

    // LOUD ON UNMEASURABLE. A walker that finds nothing exits green and is
    // indistinguishable from a clean corpus by result alone, which is the exact
    // failure this whole file exists to prevent.
    assert!(
        files > 0,
        "COULD NOT MEASURE: no .rs file in scope, the walker found no test tree"
    );
    assert!(
        !sites.is_empty(),
        "COULD NOT MEASURE: {files} file(s) scanned and zero announcement sites found. \
         The corpus has announcement sites; a zero here means the scanner broke, not \
         that the tree is clean"
    );

    let bad: Vec<&Site> = sites.iter().filter(|s| !s.text.starts_with(SKIP_MARKER)).collect();
    if !bad.is_empty() {
        let mut msg = format!(
            "{} announced early return(s) do not start with the canonical marker {SKIP_MARKER:?}.\n\
             The landing bar and scripts/nightly_source_gates.sh both match on that prefix, so a \
             site spelled any other way can no-op and still clear the zero-skip bar.\n",
            bad.len()
        );
        for s in &bad {
            let via = match (s.structural, s.lexical) {
                (true, true) => "early-return + skip vocabulary",
                (true, false) => "early-return within the structural window",
                _ => "skip vocabulary",
            };
            msg.push_str(&format!(
                "  {}:{}  [{via}]\n      {:?}\n",
                s.file.display(),
                s.line,
                s.text
            ));
        }
        panic!("{msg}");
    }

    let structural_only = sites.iter().filter(|s| s.structural && !s.lexical).count();
    let lexical_only = sites.iter().filter(|s| s.lexical && !s.structural).count();
    // THE CENSUS MUST NOT REPRODUCE THE MARKER. This line lands in the same log that
    // the landing bar and `nightly_source_gates.sh` grep for the marker, and a hit
    // there means "a gate measured nothing" — so echoing the literal here reports this
    // gate's own success as somebody else's silent skip. It did: interpolating
    // `{SKIP_MARKER:?}` made the nightly lane exit 2 (COULD NOT RUN, whole lane dark)
    // on a fully green run, which is precisely the failure this gate exists to prevent.
    // Describe the marker; never print it.
    let census = format!(
        "skip-marker census: {} announcement site(s) across {files} file(s) all carry the \
         canonical marker ({} structural-only, {lexical_only} lexical-only, {} seen by both)",
        sites.len(),
        structural_only,
        sites.len() - structural_only - lexical_only,
    );
    assert!(
        !census.contains(SKIP_MARKER),
        "this gate's own census line contains the skip marker, so a green run of it \
         trips the landing bar and takes the nightly lane dark: {census:?}"
    );
    println!("{census}");
}

/// THE TWO ENFORCERS AGREE BY CONSTRUCTION, not by two hand-maintained copies.
///
/// `scripts/nightly_source_gates.sh` must EXTRACT the marker out of the constant
/// this lint uses, so that changing the constant moves both. This test performs
/// the same extraction the script performs and checks the three agree; it also
/// refuses a script that has gone back to a retyped literal.
#[test]
fn the_nightly_script_derives_the_marker_from_the_constant() {
    let ws = crates_dir()
        .parent()
        .expect("crates/ has a parent")
        .to_path_buf();
    let script_path = ws.join("scripts/nightly_source_gates.sh");
    let script = std::fs::read_to_string(&script_path).unwrap_or_else(|e| {
        panic!(
            "COULD NOT MEASURE: {} unreadable ({e}), the second enforcer of the \
             zero-skip bar could not be checked, which is not the same as it being correct",
            script_path.display()
        )
    });

    let support_rel = "crates/sigil-harness/src/test_support.rs";
    let support = std::fs::read_to_string(ws.join(support_rel))
        .unwrap_or_else(|e| panic!("COULD NOT MEASURE: {support_rel} unreadable ({e})"));

    // The same extraction the script's sed performs, run here against the same file.
    let decl = "pub const SKIP_MARKER: &str = \"";
    let extracted = support
        .lines()
        .find_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix(decl)?;
            rest.strip_suffix("\";")
        })
        .unwrap_or_else(|| {
            panic!(
                "COULD NOT MEASURE: no extractable `{decl}…\";` line in {support_rel}. \
                 The script's sed reads that exact shape; if it is gone the script exits 2, \
                 and this test must say so rather than pass"
            )
        });
    assert_eq!(
        extracted, SKIP_MARKER,
        "the text the script's extraction yields differs from the compiled constant"
    );

    assert!(
        script.contains("SKIP_MARKER") && script.contains(support_rel),
        "{} no longer derives the marker from {support_rel}, the two enforcers of the \
         zero-skip bar have gone back to independent copies that can drift",
        script_path.display()
    );
    // A retyped literal in the script is the drift this is meant to prevent; the
    // grep must go through the extracted variable.
    for retyped in ["grep -q 'skip:'", "grep -qF 'skip:'", "grep -c 'skip:'"] {
        assert!(
            !script.contains(retyped),
            "{} still greps a retyped {retyped:?}, that copy is what drifts away from the constant",
            script_path.display()
        );
    }
}
