//! NO EM DASH OR EN DASH IN TEXT A TOOL SHOWS A PERSON.
//!
//! The owner's ruling on 2026-09-05, carried by `design/CHROME_SPEC.md` ("Text in
//! the tools") at empyrean, names sigil's diagnostics, warnings and help among the
//! surfaces it covers. Half of that ruling was a sweep, which happened once. This
//! is the other half: nobody writes a new one.
//!
//! WHAT IT ENUMERATES OVER. Every `.rs` file under `crates/`, lexed rather than
//! grepped. A line grep cannot tell a diagnostic from a note ABOUT one, and the
//! difference is not marginal here: at the sweep the workspace held 1,012
//! dash-bearing string literals and 10,840 dash-bearing comments, so a grep-based
//! gate would have been 91 percent false positives and would have been weakened
//! or deleted within a week. The lexer tracks line-comment, block-comment
//! (nested), char-literal, string-literal and raw-string state, and only a string
//! literal's body can trip it.
//!
//! WHAT IT DOES NOT COVER, stated because an assertion of completeness and the
//! check that would establish it are separable:
//!
//!   * COMMENTS, doc comments included. Out of scope by the ruling's own words,
//!     and 10,840 of them; editing them is churn that buries a real diff.
//!   * `docs/**`. The sweep was about tool text. The no-new-dashes half governs
//!     new writing there, and no mechanical check enforces it.
//!   * NON-RUST tools. `scripts/drift_report.py` prints a report a person reads,
//!     and its output text was swept alongside the Rust that asserts on it, but
//!     this gate lexes Rust and does not reach it. A Python or shell tool can
//!     still grow a dash without reddening anything here.
//!   * Text a tool BUILDS at runtime rather than writing as a literal: a dash
//!     that arrives from a data file, an environment value or a `char` computed
//!     from a codepoint is invisible to a source scan.
//!
//! WHY THE FIXTURE IS NOT DECORATION. A scanner that walks the wrong path, or
//! that silently loses its comment state, reports a clean tree in exactly the
//! words a clean tree reports. The first measurement of this population during
//! the sweep returned a confident ZERO because its pathspec matched no files.
//! So [`lexer_finds_dashes_in_strings_and_only_in_strings`] runs the same lexer
//! over a fixture whose right answer is known, and
//! [`no_em_or_en_dash_in_any_rust_string_literal`] refuses to pass on a walk that
//! found no files or no strings at all.
//!
//! This file writes its own dashes as `\u{2014}` / `\u{2013}` escapes, which are
//! ASCII in the source text, so it is scanned like every other file rather than
//! excluding itself. A self-exclusion is a hole.

use std::path::{Path, PathBuf};

/// U+2014 EM DASH.
const EM: char = '\u{2014}';
/// U+2013 EN DASH.
const EN: char = '\u{2013}';

/// A string literal found by [`string_literals`].
#[derive(Debug)]
struct Literal {
    line: usize,
    body: String,
}

/// Every string-literal BODY in `src`, with the source line it opens on.
///
/// Handles `"…"`, `r"…"`, `r#"…"#`, `b"…"` and `br#"…"#`, skips `//` and nested
/// `/* … */` comments, and steps over char literals and lifetimes.
fn string_literals(src: &str) -> Vec<Literal> {
    let b: Vec<char> = src.chars().collect();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    while i < n {
        let c = b[i];
        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < n && b[i + 1] == '/' {
            while i < n && b[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < n && b[i + 1] == '*' {
            let mut depth = 1usize;
            i += 2;
            while i < n && depth > 0 {
                if b[i] == '\n' {
                    line += 1;
                    i += 1;
                } else if b[i] == '/' && i + 1 < n && b[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if b[i] == '*' && i + 1 < n && b[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            continue;
        }
        // raw string, optionally byte-prefixed
        let mut j = i;
        if c == 'b' && j + 1 < n && b[j + 1] == 'r' {
            j += 1;
        }
        if b[j] == 'r' {
            let mut k = j + 1;
            let mut hashes = 0usize;
            while k < n && b[k] == '#' {
                hashes += 1;
                k += 1;
            }
            let starts_token = i == 0 || !(b[i - 1].is_alphanumeric() || b[i - 1] == '_');
            if k < n && b[k] == '"' && starts_token {
                let opened_on = line;
                k += 1;
                let body_start = k;
                let mut end = n;
                let mut m = k;
                while m < n {
                    if b[m] == '"' {
                        let mut h = 0usize;
                        while h < hashes && m + 1 + h < n && b[m + 1 + h] == '#' {
                            h += 1;
                        }
                        if h == hashes {
                            end = m;
                            break;
                        }
                    }
                    m += 1;
                }
                let body: String = b[body_start..end].iter().collect();
                line += body.matches('\n').count();
                out.push(Literal {
                    line: opened_on,
                    body,
                });
                i = end + 1 + hashes;
                continue;
            }
        }
        // char literal, or a lifetime, which is not one
        if c == '\'' {
            let k = i + 1;
            if k < n && b[k] == '\\' {
                let mut m = k + 2;
                while m < n && b[m] != '\'' {
                    m += 1;
                }
                i = m + 1;
                continue;
            }
            if k + 1 < n && b[k + 1] == '\'' {
                i = k + 2;
                continue;
            }
            i += 1;
            continue;
        }
        if c == '"' || (c == 'b' && i + 1 < n && b[i + 1] == '"') {
            if c == 'b' {
                i += 1;
            }
            let opened_on = line;
            i += 1;
            let mut body = String::new();
            while i < n {
                if b[i] == '\\' {
                    body.push(b[i]);
                    if i + 1 < n {
                        if b[i + 1] == '\n' {
                            line += 1;
                        }
                        body.push(b[i + 1]);
                    }
                    i += 2;
                    continue;
                }
                if b[i] == '"' {
                    break;
                }
                if b[i] == '\n' {
                    line += 1;
                }
                body.push(b[i]);
                i += 1;
            }
            out.push(Literal {
                line: opened_on,
                body,
            });
            i += 1;
            continue;
        }
        i += 1;
    }
    out
}

fn crates_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is <workspace>/crates/sigil-harness.
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .unwrap_or_else(|| panic!("no parent of {}", here.display()))
        .to_path_buf()
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if p.is_dir() {
            if name == "target" || name == "node_modules" || name.starts_with('.') {
                continue;
            }
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// The lexer's own positive control. A scan that walks nothing, or that loses
/// comment state, reports a clean tree in the same words a clean tree does; this
/// fixture has a known right answer, so only one of those two can pass it.
///
/// EACH LINE OF THE FIXTURE EARNS ITS PLACE, and two of them were added after the
/// first version failed to catch a mutation it should have. Blinding the
/// line-comment branch left this fixture green, because its comments held no
/// quote character for a blinded lexer to open a string on. The comments that
/// QUOTE a dashed phrase are the ones that turn that mutation red, and they are
/// also the shape the real tree is full of.
#[test]
fn lexer_finds_dashes_in_strings_and_only_in_strings() {
    // Built from escapes so this file stays clean under its own scan.
    let em = EM;
    let en = EN;
    let fixture = format!(
        "// a line comment with an em dash {em} must not count\n\
         // a comment that quotes \"a phrase {em} with a dash\" must not count either\n\
         /// a doc comment with an en dash {en} must not count\n\
         /// a doc comment quoting \"another phrase {en} with a dash\" must not count\n\
         /* a block comment {em} with a /* nested {en} */ tail {em} */\n\
         fn f() {{\n\
         let lifetime: &'static str = \"no dash\";\n\
         let ch = '-';\n\
         let one = \"an em dash {em} in a string\";\n\
         let two = \"an en dash {en} in a string\";\n\
         let three = r#\"raw, a quote \" and a dash {em} here\"#;\n\
         let four = \"has // a slash-slash and a dash {em} here\";\n\
         let five = \"hyphen - only\";\n\
         }}\n"
    );

    let lits = string_literals(&fixture);
    let hits: Vec<&Literal> = lits
        .iter()
        .filter(|l| l.body.contains(EM) || l.body.contains(EN))
        .collect();

    assert!(
        !lits.is_empty(),
        "COULD NOT MEASURE: the lexer found no string literals at all in the control \
         fixture, so a clean result over the workspace would mean nothing"
    );
    assert_eq!(
        hits.len(),
        4,
        "the control fixture holds exactly four dash-bearing string literals and \
         five dashes in comments plus one in a char-adjacent position; the lexer \
         found {} hit(s): {:?}",
        hits.len(),
        hits
    );
    for l in &hits {
        assert!(
            l.body.contains("in a string") || l.body.contains("here"),
            "a comment leaked into the string bucket: {:?}",
            l
        );
    }
}

/// THE GATE. No em dash and no en dash in any Rust string literal in the tree.
#[test]
fn no_em_or_en_dash_in_any_rust_string_literal() {
    let crates = crates_dir();
    assert!(
        crates.is_dir(),
        "COULD NOT MEASURE: the crates tree is not at {}, so this gate scanned \
         nothing, which is not the same as finding nothing",
        crates.display()
    );

    let mut files = Vec::new();
    collect_rs(&crates, &mut files);
    files.sort();

    // Sized off what the tree holds, not off a number typed in here: any walk
    // that lands on a fraction of the sources is a broken walk, not a clean one.
    assert!(
        files.len() >= 100,
        "COULD NOT MEASURE: the walk under {} found only {} .rs file(s). The tree \
         held 493 at the sweep; a walk this short is broken, not clean",
        crates.display(),
        files.len()
    );

    let mut literals_seen = 0usize;
    let mut findings: Vec<String> = Vec::new();
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            panic!(
                "COULD NOT MEASURE: {} could not be read, so it was not scanned",
                f.display()
            );
        };
        for lit in string_literals(&src) {
            literals_seen += 1;
            if lit.body.contains(EM) || lit.body.contains(EN) {
                let rel = f.strip_prefix(&crates).unwrap_or(f);
                let mut shown: String = lit.body.chars().take(160).collect();
                if lit.body.chars().count() > 160 {
                    shown.push_str(" …");
                }
                findings.push(format!("  crates/{}:{}  {}", rel.display(), lit.line, shown));
            }
        }
    }

    assert!(
        literals_seen >= 10_000,
        "COULD NOT MEASURE: the lexer found only {literals_seen} string literal(s) \
         across {} file(s). It lost its place; a clean verdict from this run would \
         be an artefact of the scan, not a property of the tree",
        files.len()
    );

    assert!(
        findings.is_empty(),
        "{} string literal(s) carry an em dash (U+2014) or an en dash (U+2013). \
         Text a tool shows a person uses a comma, a colon, a period or \
         parentheses instead (owner ruling 2026-09-05, design/CHROME_SPEC.md, \
         \"Text in the tools\"). Comments are exempt; these are not comments:\n{}",
        findings.len(),
        findings.join("\n")
    );
}
