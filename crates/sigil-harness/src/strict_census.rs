//! THE STRICT-GATE CENSUS — a POPULATION, not a floor.
//!
//! `refreeze --attest` records `strict_bodies`: the strict-gated decision points a run
//! actually reached with `SIGIL_STRICT_GATE` observed set. The witness behind it is
//! sound — `test_support::strict_gate` writes a line ONLY on the branch that has
//! already seen the flag set, so the count is structurally zero when the flag is unset.
//!
//! What the tool did with that number was not. It refused at zero, and zero is a FLOOR:
//! satisfiable by the very failure the mechanism exists to catch. Delete a strict-gated
//! gate, `#[ignore]` it, or drop its guard, and the witness falls from 29 to 28 —
//! comfortably above zero — and `--attest` records a pass. A gate going dark showed up
//! as a SMALLER GREEN.
//!
//! # The relation this module supplies
//!
//! A count can only be READ; a population can be DIFFED. The witness was already a
//! population — `file:line` per site, `#[track_caller]` all the way through the
//! per-file `strict_gate()` wrappers — and the tool was reducing it to `.len()` before
//! anyone looked at it. So the expectation here is a population too, DERIVED from the
//! test tree at attest time and never copied, and the assertion is a SET DIFFERENCE
//! whose diagnostic names the gate that went dark.
//!
//! Two independent detectors, because neither covers the other's set:
//!
//!   **A — DECLARED SITES.** Every `if !strict_gate() { … }` consultation the source
//!   declares must appear in the witness. Catches a gate that still EXISTS but did not
//!   EXECUTE: `#[ignore]`d, filtered out, its binary not run, or an early return above
//!   it. Names the exact `file:line` and its enclosing function.
//!
//!   **B — DECLARED TESTS.** Every `#[test]` in a file that carries at least one such
//!   consultation must appear in the witness as a reached test. Catches a gate that
//!   LOSES ITS GUARD while the test survives — invisible to A, because deleting the
//!   guard deletes A's expectation in the same edit. It also covers guards that live in
//!   a shared helper (`section_row_fixture.rs`'s `gate_on()` serves three tests through
//!   ONE site, so A cannot tell that two of the three went dark and B can).
//!
//! # Why the expectation is DERIVED and never committed
//!
//! A frozen list of names would be as brittle as the number was, for the same reason: the
//! population is legitimately variable. Retiring a strict-gated gate is an ordinary,
//! honest act, and a mechanism whose only response to one is "hand-edit the expectation"
//! teaches the reflex this repo's own `provenance::Superseded` doc argues against — the
//! honest operator forging a field. Deriving at attest time means a retirement removes
//! the gate and the expectation in the same edit, with nothing to update by hand. The
//! price is that the same property makes a DELETION invisible, which is the first
//! residual below and is why it is ledgered rather than claimed.
//!
//! Detector B keys on tests DECLARED in a gated file, not on tests that were SCHEDULED.
//! That is deliberate, and it is the one place this census can produce a red for an
//! honest reason: a test that legitimately needs no strict reference, written into a
//! gated file, has the same signature as a guard someone deleted. From outside the two
//! are indistinguishable, so the gate ASKS rather than knows — and its exit is a code
//! change (move the test, or give it its guard), never a field to forge. Measured
//! 2026-08-27: no gated file carries `#[ignore]` or any `cfg` attribute, and `--attest`
//! passes no test filter, so declared and scheduled are the same set today.
//!
//! # LOUD ON UNMEASURABLE
//!
//! A scanner that breaks and finds nothing exits green and is indistinguishable from a
//! clean tree by result alone — which is literally the defect this module closes, so it
//! must not be reintroduced here. [`census`] returns `Err` when the tree is not where it
//! should be, when it scans zero files, when it finds zero sites, and when it meets a
//! `strict_gate()` occurrence it cannot CLASSIFY. That last one matters most: a new
//! idiom the scanner does not recognise would otherwise shrink the expectation
//! silently, which is the floor defect wearing a scanner's clothes.
//!
//! # What it does NOT catch
//!
//! Stated because an assertion of completeness and the check that would establish it
//! are separable, and only the assertion is cheap:
//!
//!   * **Deleting a whole strict-gated test, or a whole gated file.** Both detectors
//!     derive from the same source, so a deletion removes the expectation in the same
//!     edit that removes the gate. Only a MEMORY of the previous population can see
//!     that; `refreeze --attest` carries a monotonic ratchet against the last recorded
//!     `strict_bodies` in the provenance chain for exactly this, and that ratchet is
//!     unarmed until the chain records its first strict run.
//!   * **A gate that runs to completion but asserts nothing.** A vacuous body reaches
//!     its guard and is counted; that is a different defect class with a different bar.
//!   * **A guard reached on a spawned thread.** libtest names the test thread; a child
//!     thread it spawns is anonymous, so B would read the test as dark. No gate does
//!     this today, and the defect message names the possibility.
//!
//! Those residuals are ledgered in `docs/superpowers/notes/campaign-gap-ledger.md`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The thread name recorded when the guard did not run on a libtest test thread.
pub const UNNAMED_THREAD: &str = "-";

/// One DECLARED strict-gate consultation — detector A's population.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// Workspace-relative, matching what `Location::file()` reports.
    pub file: String,
    pub line: usize,
    /// The function the consultation is written in — the test itself, or the helper
    /// that gates on its behalf.
    pub owner: String,
    /// Whether [`Self::owner`] is a `#[test]` rather than a helper.
    pub owner_is_test: bool,
}

impl Site {
    /// The witness key: exactly what `strict_gate`'s witness writes.
    pub fn key(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }
}

/// One `#[test]` in a strict-gated file — detector B's population.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestFn {
    pub file: String,
    pub name: String,
    pub line: usize,
    /// `#[ignore]`d. Kept IN the population on purpose: `#[ignore]` on a strict-gated
    /// test is a gate going dark, which is the failure this whole module is about.
    /// Retiring one is meant to cost removing its guard, not one attribute.
    pub ignored: bool,
}

impl TestFn {
    /// The witness key: `(site file, test name)`.
    pub fn key(&self) -> (String, String) {
        (self.file.clone(), self.name.clone())
    }
}

/// How a `strict_gate()` occurrence was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// `fn strict_gate() -> bool` / the per-file wrapper's delegating body. Not a site.
    Plumbing,
    /// `if !strict_gate() { … }` — consulted UNCONDITIONALLY, so a healthy strict run
    /// reaches it. Detector A's population.
    Unconditional,
    /// `if strict_gate() { panic!(…) }`, `Err(_) if strict_gate() => panic!(…)`,
    /// `assert!(!strict_gate(), …)` — consulted only on a MISSING-reference path. A
    /// healthy strict run never reaches these, and under strict they panic when it
    /// does, so counting them would overstate the expectation and go permanently red.
    MissingReferencePath,
}

/// The derived expectation.
#[derive(Debug, Clone, Default)]
pub struct Census {
    pub files_scanned: usize,
    /// Detector A: every unconditional consultation.
    pub sites: Vec<Site>,
    /// Detector B: every `#[test]` in a file that carries at least one site.
    pub tests: Vec<TestFn>,
    /// Occurrences classified [`Class::MissingReferencePath`] — reported so the census
    /// can say how much of the corpus it deliberately excluded.
    pub missing_reference_paths: usize,
    /// Occurrences classified [`Class::Plumbing`].
    pub plumbing: usize,
}

/// What one run actually reached.
#[derive(Debug, Clone, Default)]
pub struct Witness {
    /// Distinct `file:line`. This IS `strict_bodies`.
    pub sites: BTreeSet<String>,
    /// Distinct `(file, test name)`.
    pub pairs: BTreeSet<(String, String)>,
    /// Total lines, before dedup — sites vs executions.
    pub raw_lines: usize,
}

/// Normalise a path so a workspace-relative witness line and an absolute census path
/// compare equal: both are keyed from `crates/` onward.
fn norm(path: &str) -> String {
    match path.find("crates/") {
        Some(i) => path[i..].to_string(),
        None => path.to_string(),
    }
}

/// Read one strict-run witness file's text into a population.
///
/// Line format: `file:line` optionally followed by a TAB and the libtest thread name,
/// which is the test's own name (measured: libtest names each test thread after the
/// test, module path included, under `--test-threads=1` as well). A line without the
/// tab is an older witness and yields [`UNNAMED_THREAD`], which detector B reports
/// rather than silently accepting.
pub fn parse_witness(text: &str) -> Witness {
    let mut w = Witness::default();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        w.raw_lines += 1;
        let (site, test) = match line.split_once('\t') {
            Some((s, t)) => (s.trim(), t.trim()),
            None => (line, UNNAMED_THREAD),
        };
        let site = norm(site);
        let file = match site.rfind(':') {
            Some(i) => site[..i].to_string(),
            None => site.clone(),
        };
        w.sites.insert(site);
        w.pairs.insert((file, test.to_string()));
    }
    w
}

/// The code half of a source line: everything before a `//` that is not inside a string
/// literal. Prose about `strict_gate()` — and this crate has a lot of it — must not be
/// mistaken for a call site.
fn code_of(line: &str) -> &str {
    let b = line.as_bytes();
    let mut in_str = false;
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b'/' if !in_str && i + 1 < b.len() && b[i + 1] == b'/' => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

/// Strip any `path::` qualification off the front of a `strict_gate()` consultation.
fn after_path(s: &str) -> &str {
    match s.rfind("::") {
        Some(i) if s[..i].chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':') => {
            &s[i + 2..]
        }
        _ => s,
    }
}

/// Classify every `strict_gate()` occurrence on one code line.
///
/// Returns one class per occurrence. `prev` is the few preceding code lines, needed for
/// the multi-line `assert!(\n    !strict_gate(),\n …)` spelling in `test_support.rs`.
fn classify_line(code: &str, prev: &[&str]) -> Vec<Option<Class>> {
    let occurrences = code.matches("strict_gate()").count();
    if occurrences == 0 {
        return Vec::new();
    }
    let t = code.trim();

    // The definition and the per-file delegating wrapper. One line can hold both
    // (`fn strict_gate() -> bool { …::strict_gate() }`), so this consumes the line.
    if t.contains("fn strict_gate()") {
        return vec![Some(Class::Plumbing); occurrences];
    }
    // A wrapper body on its own line: the delegating call and nothing else.
    if after_path(t).trim_end_matches(&[';', ','][..]) == "strict_gate()" && !t.starts_with('!') {
        return vec![Some(Class::Plumbing); occurrences];
    }

    // UNCONDITIONAL — the skip-and-return guard. `if !<path>strict_gate() {`.
    if let Some(rest) = t.strip_prefix("if !") {
        if after_path(rest).starts_with("strict_gate()") {
            let tail = after_path(rest)["strict_gate()".len()..].trim_start();
            if tail.starts_with('{') {
                return vec![Some(Class::Unconditional); occurrences];
            }
        }
    }

    // MISSING-REFERENCE PATHS.
    //  * `if <path>strict_gate() {` — the panic-under-strict branch of a skip.
    if let Some(rest) = t.strip_prefix("if ") {
        if after_path(rest).starts_with("strict_gate()") {
            return vec![Some(Class::MissingReferencePath); occurrences];
        }
    }
    //  * a match guard: `Err(_) if strict_gate() => panic!(…)`.
    if t.contains(" if ") && t.contains("=>") {
        return vec![Some(Class::MissingReferencePath); occurrences];
    }
    //  * `assert!(!strict_gate(), …)`, on one line or wrapped onto the next.
    let negated = t.starts_with('!') || t.contains("(!");
    let asserted = t.contains("assert!(") || prev.iter().rev().any(|p| p.trim().ends_with("assert!("));
    if negated && asserted {
        return vec![Some(Class::MissingReferencePath); occurrences];
    }

    vec![None; occurrences]
}

/// Every `.rs` under `dir`, sorted, recursively.
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

/// Every file whose `strict_gate()` consultations can land in a strict suite log: the
/// whole test tree, plus the shared guard helper that consults on the tests' behalf.
///
/// `crates_dir` is `<workspace>/crates`.
pub fn scanned_files(crates_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !crates_dir.is_dir() {
        return Err(format!(
            "COULD NOT MEASURE: the crates tree is not at {} — the census scanned nothing, \
             which is not the same as finding nothing",
            crates_dir.display()
        ));
    }
    let mut crate_dirs: Vec<PathBuf> = std::fs::read_dir(crates_dir)
        .map_err(|e| format!("COULD NOT MEASURE: read {}: {e}", crates_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    crate_dirs.sort();

    let mut out = Vec::new();
    for c in crate_dirs {
        collect_rs(&c.join("tests"), &mut out);
    }
    let helper = crates_dir.join("sigil-harness/src/test_support.rs");
    if helper.is_file() {
        out.push(helper);
    }
    // This module's own lint quotes every idiom it classifies, in code as well as prose.
    out.retain(|p| p.file_name().and_then(|n| n.to_str()) != Some("strict_census_lint.rs"));
    out.sort();
    Ok(out)
}

/// DERIVE the expectation from the tree. Never a constant, never a copy.
pub fn census(crates_dir: &Path) -> Result<Census, String> {
    let root = crates_dir
        .parent()
        .ok_or_else(|| format!("COULD NOT MEASURE: {} has no parent", crates_dir.display()))?;
    let files = scanned_files(crates_dir)?;

    let mut c = Census {
        files_scanned: files.len(),
        ..Census::default()
    };
    let mut unclassified: Vec<String> = Vec::new();
    let mut nested_mods: Vec<String> = Vec::new();

    for path in &files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let src = std::fs::read_to_string(path).map_err(|e| {
            format!(
                "COULD NOT MEASURE: {} is in scope but unreadable ({e})",
                path.display()
            )
        })?;
        let lines: Vec<&str> = src.lines().collect();
        let code: Vec<&str> = lines.iter().map(|l| code_of(l)).collect();

        // Pass 1 — sites, and the enclosing function of each.
        let mut sites_here: Vec<Site> = Vec::new();
        let mut cur_fn = String::from("<file scope>");
        let mut cur_fn_is_test = false;
        let mut pending_test = false;
        let mut pending_ignore = false;
        let mut tests_here: Vec<TestFn> = Vec::new();
        let mut mods_here: Vec<String> = Vec::new();

        for (i, raw) in code.iter().enumerate() {
            let t = raw.trim();
            if raw.starts_with("#[test]") {
                pending_test = true;
            } else if raw.starts_with("#[ignore") {
                pending_ignore = true;
            } else if raw.starts_with("mod ") && t.contains('{') {
                mods_here.push(format!("{rel}:{}", i + 1));
            } else if let Some(name) = fn_name_at_column_zero(raw) {
                cur_fn = name.clone();
                cur_fn_is_test = pending_test;
                if pending_test {
                    tests_here.push(TestFn {
                        file: rel.clone(),
                        name,
                        line: i + 1,
                        ignored: pending_ignore,
                    });
                }
                pending_test = false;
                pending_ignore = false;
            }

            let prev_start = i.saturating_sub(3);
            for (n, class) in classify_line(raw, &code[prev_start..i]).into_iter().enumerate() {
                match class {
                    Some(Class::Plumbing) => c.plumbing += 1,
                    Some(Class::MissingReferencePath) => c.missing_reference_paths += 1,
                    Some(Class::Unconditional) => sites_here.push(Site {
                        file: rel.clone(),
                        line: i + 1,
                        owner: cur_fn.clone(),
                        owner_is_test: cur_fn_is_test,
                    }),
                    None => unclassified.push(format!(
                        "{rel}:{} (occurrence {}) — {:?}",
                        i + 1,
                        n + 1,
                        t
                    )),
                }
            }
        }

        // Detector B's population is only the GATED files: those carrying at least one
        // unconditional consultation. A file full of missing-reference guards (every
        // `*_port.rs`) has no test that must reach one, and demanding it would be a
        // permanently red gate — the 4x overstatement this census exists to avoid.
        if !sites_here.is_empty() {
            // The nested-module refusal is scoped to GATED files for the same reason
            // detector B is: only there does a bare test name have to be a witness key.
            // Every `*_port.rs` and `test_support.rs` carries `mod` blocks that no
            // detector reads, and refusing on those would be a permanently red gate.
            nested_mods.extend(mods_here);
            c.tests.extend(tests_here);
            c.sites.extend(sites_here);
        }
    }

    if c.files_scanned == 0 {
        return Err(
            "COULD NOT MEASURE: no .rs file in scope — the census walked no test tree".to_string(),
        );
    }
    if !nested_mods.is_empty() {
        // A `#[test]` inside a `mod` is named `mod::test` by libtest, and detector B
        // keys on the bare name. Rather than mis-derive, say so.
        return Err(format!(
            "COULD NOT MEASURE: {} nested module(s) in scanned files — libtest names a test \
             inside a module `mod::name`, which detector B's bare-name key would miss. Teach \
             the census the module path before adding one:\n  {}",
            nested_mods.len(),
            nested_mods.join("\n  ")
        ));
    }
    if !unclassified.is_empty() {
        return Err(format!(
            "COULD NOT MEASURE: {} `strict_gate()` occurrence(s) the census cannot CLASSIFY. \
             An unrecognised idiom shrinks the expectation silently, which is the floor defect \
             wearing a scanner's clothes — so it refuses instead. Teach `classify_line` the \
             shape, or write the guard in the established one:\n  {}",
            unclassified.len(),
            unclassified.join("\n  ")
        ));
    }
    if c.sites.is_empty() {
        return Err(format!(
            "COULD NOT MEASURE: {} file(s) scanned and ZERO unconditional `if !strict_gate()` \
             site(s) found. The corpus has them; a zero here means the census broke, not that \
             the tree is clean",
            c.files_scanned
        ));
    }
    c.sites.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    c.tests.sort_by(|a, b| (&a.file, &a.name).cmp(&(&b.file, &b.name)));
    Ok(c)
}

/// `fn NAME(` / `pub fn NAME(` at column zero — these files are flat, and the nested-mod
/// refusal above is what keeps that true.
fn fn_name_at_column_zero(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("pub fn ")
        .or_else(|| line.strip_prefix("fn "))?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// THE ASSERTION. Every way the run's population differs from the declared one.
/// Empty = the run reached exactly what the tree declares.
pub fn defects(c: &Census, w: &Witness) -> Vec<String> {
    let mut d = Vec::new();

    let reached: BTreeSet<String> = w.sites.iter().map(|s| norm(s)).collect();
    let declared: BTreeSet<String> = c.sites.iter().map(|s| norm(&s.key())).collect();

    // A — DECLARED BUT DARK. The load-bearing direction: the tree says this gate
    // consults the strict flag, and the run never got there.
    for s in &c.sites {
        if !reached.contains(&norm(&s.key())) {
            d.push(format!(
                "DARK GATE: {} declares a strict-gate consultation in `{}`{}, and the run never \
                 reached it. The gate did not measure anything this attestation would be about \
                 (`#[ignore]`, a filter, a binary that did not run, or an early return above it)",
                s.key(),
                s.owner,
                if s.owner_is_test { "" } else { " (a helper)" }
            ));
        }
    }
    // A' — REACHED BUT UNDECLARED. Not a coverage loss, a MODEL failure: the census is
    // predicting the wrong population, so its silence about everything else is worthless.
    for r in &reached {
        if !declared.contains(r) {
            d.push(format!(
                "UNDECLARED SITE: the run reached a strict-gated body at {r} that the census did \
                 not predict. The census model is wrong, so its coverage claim cannot be trusted"
            ));
        }
    }

    // B — DECLARED TEST THAT REACHED NO GATE. Catches the guard being deleted from a
    // test that survives, which A cannot see: that edit removes A's expectation too.
    let reached_pairs: BTreeSet<(String, String)> = w
        .pairs
        .iter()
        .map(|(f, t)| (norm(f), t.clone()))
        .collect();
    for t in &c.tests {
        let key = (norm(&t.file), t.name.clone());
        if !reached_pairs.contains(&key) {
            d.push(format!(
                "UNGUARDED TEST: `{}` in {} is a test in a strict-gated file, and the run records \
                 no strict-gate consultation from it{}. Either its guard was removed, it did not \
                 run, or it consulted the gate from a spawned thread",
                t.name,
                t.file,
                if t.ignored { " (it is `#[ignore]`d)" } else { "" }
            ));
        }
    }
    // B' — an unnamed thread reached a gate. The pair key is useless, so B is blind for
    // whatever test that was; say so rather than let it pass as covered.
    if w.pairs.iter().any(|(_, t)| t == UNNAMED_THREAD) {
        d.push(format!(
            "UNNAMED REACH: at least one witness line carries no test name ({UNNAMED_THREAD}). \
             Either the witness predates the test-name field, or a guard ran on a spawned \
             thread; either way detector B cannot see which test it belonged to"
        ));
    }
    d
}

/// One line describing what the census expected and what the run reached.
pub fn summary(c: &Census, w: &Witness) -> String {
    format!(
        "strict-gate census: {} declared site(s) and {} declared test(s) across {} scanned \
         file(s); the run reached {} site(s) from {} test(s) in {} witness line(s) \
         ({} missing-reference path(s) deliberately excluded)",
        c.sites.len(),
        c.tests.len(),
        c.files_scanned,
        w.sites.len(),
        w.pairs.len(),
        w.raw_lines,
        c.missing_reference_paths,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_of_ignores_prose_about_the_idiom() {
        assert_eq!(code_of("    // if !strict_gate() { … }").trim(), "");
        assert_eq!(code_of("/// every `strict_gate()`-guarded body").trim(), "");
        assert_eq!(
            code_of("    if !strict_gate() { // the guard").trim(),
            "if !strict_gate() {"
        );
        // A `//` inside a string literal is not a comment.
        assert!(code_of(r#"    let p = "http://x"; if !strict_gate() {"#).contains("strict_gate"));
    }

    fn class1(line: &str) -> Option<Class> {
        let v = classify_line(code_of(line), &[]);
        assert_eq!(v.len(), 1, "expected one occurrence in {line:?}");
        v[0]
    }

    #[test]
    fn the_four_corpus_idioms_classify() {
        assert_eq!(
            class1("    if !strict_gate() {"),
            Some(Class::Unconditional)
        );
        assert_eq!(
            class1("    if !sigil_harness::test_support::strict_gate() {"),
            Some(Class::Unconditional)
        );
        assert_eq!(
            class1("        if strict_gate() {"),
            Some(Class::MissingReferencePath)
        );
        assert_eq!(
            class1("        Err(_) if strict_gate() => panic!(\"missing\"),"),
            Some(Class::MissingReferencePath)
        );
        assert_eq!(
            class1("        assert!(!test_support::strict_gate(), \"{msg}\");"),
            Some(Class::MissingReferencePath)
        );
        assert_eq!(
            class1("    sigil_harness::test_support::strict_gate()"),
            Some(Class::Plumbing)
        );
        // The wrapped `assert!(` spelling needs the lookback.
        assert_eq!(
            classify_line(code_of("            !strict_gate(),"), &["        assert!("]),
            vec![Some(Class::MissingReferencePath)]
        );
    }

    #[test]
    fn a_shape_the_census_does_not_know_is_unclassified_not_ignored() {
        // The whole point: an idiom nobody taught it must be LOUD, never uncounted.
        assert_eq!(class1("    let on = strict_gate();"), None);
    }

    #[test]
    fn the_definition_line_holding_both_occurrences_is_plumbing() {
        let v = classify_line(
            code_of("fn strict_gate() -> bool { sigil_harness::test_support::strict_gate() }"),
            &[],
        );
        assert_eq!(v, vec![Some(Class::Plumbing), Some(Class::Plumbing)]);
    }

    #[test]
    fn the_witness_is_sites_not_executions() {
        // One site reached by three tests: three raw lines, ONE site, THREE pairs.
        let w = parse_witness(
            "crates/a/tests/f.rs:10\tone\n\
             crates/a/tests/f.rs:10\ttwo\n\
             crates/a/tests/f.rs:10\tthree\n",
        );
        assert_eq!(w.raw_lines, 3);
        assert_eq!(w.sites.len(), 1);
        assert_eq!(w.pairs.len(), 3);
    }

    #[test]
    fn a_witness_line_without_a_test_name_is_reported_not_swallowed() {
        let w = parse_witness("crates/a/tests/f.rs:10\n");
        assert_eq!(w.pairs.iter().next().unwrap().1, UNNAMED_THREAD);
        let c = Census {
            files_scanned: 1,
            sites: vec![Site {
                file: "crates/a/tests/f.rs".into(),
                line: 10,
                owner: "t".into(),
                owner_is_test: true,
            }],
            tests: vec![],
            ..Census::default()
        };
        let d = defects(&c, &w);
        assert!(d.iter().any(|m| m.starts_with("UNNAMED REACH")), "{d:?}");
    }

    fn fixture() -> (Census, Witness) {
        let c = Census {
            files_scanned: 1,
            sites: vec![Site {
                file: "crates/a/tests/f.rs".into(),
                line: 10,
                owner: "alpha".into(),
                owner_is_test: true,
            }],
            tests: vec![TestFn {
                file: "crates/a/tests/f.rs".into(),
                name: "alpha".into(),
                line: 9,
                ignored: false,
            }],
            ..Census::default()
        };
        let w = parse_witness("crates/a/tests/f.rs:10\talpha\n");
        (c, w)
    }

    #[test]
    fn a_matching_population_has_no_defects() {
        let (c, w) = fixture();
        assert_eq!(defects(&c, &w), Vec::<String>::new());
    }

    #[test]
    fn a_dark_gate_is_named_not_counted() {
        let (c, _) = fixture();
        let d = defects(&c, &parse_witness(""));
        assert!(d.iter().any(|m| m.contains("DARK GATE") && m.contains("f.rs:10")), "{d:?}");
    }

    #[test]
    fn a_test_that_lost_its_guard_is_caught_by_detector_b() {
        // The guard is gone from source, so detector A's expectation went with it —
        // this is the case A structurally cannot see.
        let (mut c, _) = fixture();
        c.sites.clear();
        let d = defects(&c, &parse_witness(""));
        assert!(d.iter().any(|m| m.contains("UNGUARDED TEST") && m.contains("alpha")), "{d:?}");
    }

    #[test]
    fn an_absolute_witness_path_still_compares() {
        let (c, _) = fixture();
        let w = parse_witness("/home/x/sigil/crates/a/tests/f.rs:10\talpha\n");
        assert_eq!(defects(&c, &w), Vec::<String>::new());
    }
}
