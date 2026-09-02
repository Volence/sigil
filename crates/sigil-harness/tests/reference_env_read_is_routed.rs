//! NO TEST READS THE REFERENCE-TREE VARIABLES FOR ITSELF.
//!
//! The `d-18` refusal is centralised: `test_support::aeon_dir` is the one function that
//! commits a gate to a tree, and it is where a run that named no tree STOPS instead of
//! going green over the rows it did not measure. `test_support::partial_run_banner` is
//! where a declared partial run gets its DERIVED not-measured count.
//!
//! Both live behind one door, and a test can walk around it in about eight words:
//!
//! ```text
//! let aeon = PathBuf::from(std::env::var(<the checkout variable>).unwrap());
//! ```
//!
//! That line raises no refusal, enters no not-measured count, and — with nothing named —
//! silently resolves the owner's live working checkout, whose revision moves under a run.
//! A result measured against it is attributable to whatever that tree happened to contain.
//! That is precisely the state `d-18` exists to end, reachable by not using the door.
//!
//! ## The rule, and why it is a location and not a list
//!
//! **A file under `crates/*/tests/` may not read a reference-tree variable. The resolver
//! lives in `crates/sigil-harness/src/`.** Sanctioned readers are identified by where they
//! are, so there is no roster of blessed callers to keep — and therefore no baseline whose
//! remedy, when it goes red, is a hand edit indistinguishable from hiding the defect. The
//! population has no exceptions today and the gate has no mechanism for one.
//!
//! Two things a test legitimately does with these variables stay legal, and neither is a
//! read:
//!
//!   * SETTING them for a child process (`Command::env` / `env_remove`) — that is how the
//!     precedence gates and the write guards arrange the environment they measure;
//!   * asking the harness a QUESTION about them — `checkout_var_is_set`,
//!     `aeon_dir_is_unnamed`, `aeon_checkout` — which is the door.
//!
//! ## What it is derived from
//!
//! Nothing here is typed. The variable NAMES come from the two published constants the
//! resolver's precedence steps 1 and 2 are declared with, linked as Rust items rather than
//! respelled; the constants' own IDENTIFIERS are read back out of `test_support.rs` by
//! matching those values, so a read routed through the constant is caught by the same rule
//! as a read spelling the literal. The population is every `.rs` under `crates/*/tests/`,
//! walked from this workspace's own root.
//!
//! ## How it tells "no violations" from "nothing was examined"
//!
//! The two are identical from a green result and the second is the cheaper accident, so
//! three controls, each `UNMEASURABLE` rather than a pass:
//!
//!   * the detector is run against a SYNTHETIC violation and must report it, and against
//!     the same text inside a comment and must not — so a green means the detector works,
//!     not merely that it ran;
//!   * every constant the rule is derived from must have resolved to a non-empty,
//!     distinct name;
//!   * the walk must reconcile with an INDEPENDENT derivation of the same tree —
//!     `reference_dependence::reference_dependent_binaries` — so a walk that quietly found
//!     a fraction of the files cannot report a clean sweep.
//!
//! ## THE HOLES THIS LEAVES, stated
//!
//!   * A THIRD tree-naming variable. The rule covers precedence steps 1 and 2, which are
//!     the two the contract declares. A new variable added to the precedence would need a
//!     line here; until then a read of it would pass unseen.
//!   * `#[cfg(test)]` modules under `crates/*/src/`. They are inside the sanctioned
//!     location by this rule's own definition — that is what makes the door a door — so a
//!     private read written there is out of scope.
//!   * The rule matches the CALL SPELLING, so a read reached through an alias — `use
//!     std::env::var;` and then a bare `var(…)` — is not seen. Nothing in the tree does
//!     that, and it is not the shape the bypass takes, which is a fully qualified call
//!     pasted into a gate that wanted a path.
//!   * Comment stripping is a scanner, not a Rust lexer: a `/*` or `//` inside a string
//!     literal desynchronises it. Both directions of that error blank out code rather than
//!     manufacturing it, so it can hide a violation and cannot invent one.
//!
//! It needs no reference tree and never skips: it reads this repo's own sources and runs
//! in every `cargo test --workspace`, which is what `scripts/landing-run.sh` invokes.

use sigil_harness::reference_dependence::{reference_dependent_binaries, workspace_root, FLOOR};
use sigil_harness::test_support::{AEON_DIR_VAR, SUITE_ROOT_VAR};
use std::path::{Path, PathBuf};

/// The harness file the resolver lives in, and the file the constant identifiers are read
/// back out of.
const SUPPORT_RS: &str = "crates/sigil-harness/src/test_support.rs";

/// The call spellings that touch an environment variable IN THIS PROCESS.
///
/// The two reads are the bypass this gate is named for. The two mutators are here because
/// they are the same bypass with a mutation attached: a test that sets the checkout
/// variable in-process changes what every other row in the same binary resolves, across
/// libtest's threads, and nothing downstream can tell that tree from one an operator named.
/// Setting the variable on a CHILD is `Command::env` / `env_remove` — a different API on a
/// different receiver, matched by none of these, and it stays legal.
const TOUCHES: [&str; 4] =
    ["env::var(", "env::var_os(", "env::set_var(", "env::remove_var("];

/// Rust source with comments removed and whitespace collapsed, each surviving character
/// carrying the line it came from.
///
/// Collapsing whitespace is what makes the match independent of formatting: a read split
/// across lines by `rustfmt`, or padded inside its parentheses, is the same text here.
struct Code {
    chars: Vec<char>,
    line_of: Vec<usize>,
}

impl Code {
    fn of(src: &str) -> Code {
        let b: Vec<char> = src.chars().collect();
        let mut chars = Vec::new();
        let mut line_of = Vec::new();
        let (mut i, mut line, mut depth) = (0usize, 1usize, 0usize);
        while i < b.len() {
            let c = b[i];
            if c == '\n' {
                line += 1;
                i += 1;
                continue;
            }
            let next = b.get(i + 1).copied();
            if c == '/' && next == Some('*') {
                depth += 1;
                i += 2;
                continue;
            }
            if depth > 0 {
                if c == '*' && next == Some('/') {
                    depth -= 1;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }
            if c == '/' && next == Some('/') {
                while i < b.len() && b[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if !c.is_whitespace() {
                chars.push(c);
                line_of.push(line);
            }
            i += 1;
        }
        Code { chars, line_of }
    }

    /// Every index at which `needle` occurs.
    fn find_all(&self, needle: &str) -> Vec<usize> {
        let n: Vec<char> = needle.chars().collect();
        let mut out = Vec::new();
        if n.is_empty() || self.chars.len() < n.len() {
            return out;
        }
        for i in 0..=self.chars.len() - n.len() {
            if self.chars[i..i + n.len()] == n[..] {
                out.push(i);
            }
        }
        out
    }

    /// The text from `at` up to the next `)`, or to the end — a call's argument region.
    fn argument_at(&self, at: usize) -> String {
        self.chars[at..].iter().take_while(|c| **c != ')').collect()
    }

    fn line_at(&self, at: usize) -> usize {
        self.line_of.get(at).copied().unwrap_or(0)
    }
}

/// One private read: where it is and what it named.
struct Violation {
    file: String,
    line: usize,
    argument: String,
}

/// Every private read of a reference-tree variable in `src`, by the rule above.
///
/// `tokens` are the argument spellings that make a read a reference-tree read — the quoted
/// variable names and the identifiers of the constants carrying them.
fn violations_in(label: &str, src: &str, tokens: &[String]) -> Vec<Violation> {
    let code = Code::of(src);
    let mut out = Vec::new();
    for spelling in TOUCHES {
        for at in code.find_all(spelling) {
            let arg = code.argument_at(at + spelling.chars().count());
            if tokens.iter().any(|t| arg.contains(t.as_str())) {
                out.push(Violation {
                    file: label.to_string(),
                    line: code.line_at(at),
                    argument: arg,
                });
            }
        }
    }
    out
}

/// The identifier `test_support.rs` declares `value` under, read out of that file.
///
/// Derived rather than typed so a read routed through the constant is caught by the same
/// rule as one spelling the literal, and so a renamed constant is a red here rather than a
/// quietly narrower rule.
fn const_named(code: &Code, value: &str) -> Option<String> {
    // Whitespace is collapsed, so the declaration reads `pubconst<IDENT>:&str="<value>";`
    // and the identifier is what lies between the last `const` and the type ascription.
    let kw: Vec<char> = "const".chars().collect();
    for at in code.find_all(&format!(":&str=\"{value}\";")) {
        let start = (0..at.saturating_sub(kw.len()) + 1)
            .rev()
            .find(|i| code.chars[*i..*i + kw.len()] == kw[..])
            .map(|i| i + kw.len())?;
        let ident: String = code.chars[start..at].iter().collect();
        if !ident.is_empty() && ident.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Some(ident);
        }
    }
    None
}

/// Every `.rs` file under `crates/*/tests/`, one level down and one level deeper (the
/// shared helper modules), as (label, absolute path).
fn test_sources(ws: &Path) -> Vec<(String, PathBuf)> {
    fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                rs_files(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    for crate_dir in std::fs::read_dir(ws.join("crates")).into_iter().flatten().flatten() {
        let mut found = Vec::new();
        rs_files(&crate_dir.path().join("tests"), &mut found);
        for p in found {
            let label = p.strip_prefix(ws).unwrap_or(&p).display().to_string();
            out.push((label, p));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn no_test_reads_a_reference_tree_variable_for_itself() {
    let ws = workspace_root();

    let support = ws.join(SUPPORT_RS);
    let support_src = std::fs::read_to_string(&support).unwrap_or_else(|e| {
        panic!(
            "UNMEASURABLE: {} is unreadable ({e}). The rule's spellings are derived from it, so \
             without it this gate cannot tell a routed call from a private read.",
            support.display()
        )
    });
    let support_code = Code::of(&support_src);

    // THE SPELLINGS, derived. The values come from the published constants as Rust items;
    // the identifiers come back out of the harness source by matching those values.
    let mut tokens: Vec<String> = Vec::new();
    for value in [AEON_DIR_VAR, SUITE_ROOT_VAR] {
        assert!(
            !value.is_empty(),
            "UNMEASURABLE: a reference-tree constant is the empty string, so its spelling would \
             match every call and this gate would be about nothing."
        );
        tokens.push(format!("\"{value}\""));
        let ident = const_named(&support_code, value).unwrap_or_else(|| {
            panic!(
                "UNMEASURABLE: no `pub const … : &str = \"{value}\";` is declared in {SUPPORT_RS}, \
                 so the constant a read could be routed through cannot be named. A read spelled \
                 through it would pass this gate unseen."
            )
        });
        tokens.push(ident);
    }
    assert_eq!(
        tokens.len(),
        4,
        "UNMEASURABLE: expected a quoted name and a constant identifier for each of the two \
         precedence variables; derived {tokens:?}"
    );
    assert_ne!(
        AEON_DIR_VAR, SUITE_ROOT_VAR,
        "UNMEASURABLE: the two precedence variables resolved to the same name, so the rule covers \
         one step and reads as if it covered two."
    );

    // THE DETECTOR, proven live on this run rather than assumed. A green below means these
    // two came out right, so a detector that had stopped matching cannot report a clean
    // tree.
    //
    // The buffers are ASSEMBLED from a value carried in a variable, never written out as a
    // call this file's own text contains. That is not tidiness: it is what lets this file
    // stay INSIDE the population it scans. A gate that had to exclude itself to pass would
    // be the one test source in the repo free to do the thing it forbids.
    let quoted = format!("\"{AEON_DIR_VAR}\"");
    let planted = format!("fn f() {{ let _ = std::env::var({quoted}); }}\n");
    assert_eq!(
        violations_in("<control>", &planted, &tokens).len(),
        1,
        "UNMEASURABLE: the detector did not find a read it was handed directly:\n{planted}"
    );
    let commented = format!("// let _ = std::env::var({quoted});\nfn f() {{}}\n");
    assert!(
        violations_in("<control>", &commented, &tokens).is_empty(),
        "UNMEASURABLE: the detector reported a violation inside a comment, so it cannot tell the \
         rule from prose about the rule and would fire on files that break nothing:\n{commented}"
    );

    // THE POPULATION, and its reconciliation against an independent walk of the same tree.
    let files = test_sources(&ws);
    assert!(
        files.len() > FLOOR,
        "UNMEASURABLE: the walk of {}/crates/*/tests found only {} source files (floor {}), so a \
         clean result says nothing about the test tree.",
        ws.display(),
        files.len(),
        FLOOR
    );
    let scanned: Vec<String> = files
        .iter()
        .filter_map(|(_, p)| p.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .collect();
    let missed: Vec<String> = reference_dependent_binaries(&ws)
        .into_iter()
        .filter(|b| !scanned.contains(b))
        .collect();
    assert!(
        missed.is_empty(),
        "UNMEASURABLE: {} test binaries that an independent derivation calls reference-dependent \
         are not in this gate's walk at all: {missed:?}. Two walks of one tree disagreeing means \
         this one is scanning a subset and reporting on the whole.",
        missed.len()
    );

    let mut found = Vec::new();
    for (label, path) in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            panic!(
                "UNMEASURABLE: {label} is in the test tree but unreadable, so this run cannot say \
                 whether it reads the reference tree privately."
            )
        };
        found.extend(violations_in(label, &text, &tokens));
    }

    let report: Vec<String> = found
        .iter()
        .map(|v| format!("{}:{} — env read of {}", v.file, v.line, v.argument))
        .collect();
    assert!(
        report.is_empty(),
        "{} test source(s) read a reference-tree variable directly:\n  {}\n\nA private read walks \
         around `test_support::aeon_dir`, which is where a run that named no tree REFUSES (d-18) \
         and where a declared partial run gets its derived not-measured count. Unnamed, such a \
         read resolves the owner's live working checkout, whose revision moves under a run — the \
         state the refusal exists to end, reached by not using the door.\n\nUse the door: \
         `test_support::aeon_dir` for a tree to measure against, `aeon_checkout` for the \
         checkout and the step that answered, `checkout_var_is_set` / `aeon_dir_is_unnamed` \
         for a question about the environment. Setting the variable on a CHILD \
         (`Command::env` / `env_remove`) is not a read and is unaffected.\n\nThe accessor \
         names above are written WITHOUT their parentheses on purpose: \
         `scripts/nightly_source_gates.sh` classifies a test file by whether its code text \
         calls one, and a file that merely NAMES an accessor in a message would be read as \
         a file that obtains a reference tree — which would leave it in neither of that \
         lane's buckets and make the whole lane refuse to run.",
        report.len(),
        report.join("\n  ")
    );

    println!(
        "{} test sources scanned under crates/*/tests, {} spellings ({}), 0 private reads",
        files.len(),
        tokens.len(),
        tokens.join(" ")
    );
}
