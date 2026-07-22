//! Parcel 8b soundness guard (rider R3) — the `Block_Stage_Keys` toucher census.
//!
//! The FindStagedBlock scan memoize (`Pfx_Memo_*` / `Cs_Memo_*`, keyed on the
//! `Block_Stage_Gen` generation word) is behavior-preserving ONLY while every path
//! that records a staging key bumps the generation. A staging key is recorded by
//! writing `Block_Stage_Keys`, and the gen-bump audit that sanctioned the memo
//! rested on there being EXACTLY three procs that name that symbol:
//!
//!   * `TileCache_FindStagedBlock`   — probe / read-only
//!   * `TileCache_InvalidateStaging` — sentinel-write (bumps gen)
//!   * `TileCache_DecompressBlock`   — round-robin claim/record (bumps gen)
//!
//! A future FOURTH toucher would be a claim path the gen-bump audit never saw — a
//! silent soundness hole (a memo could survive a frame in which a block was staged
//! without the gen changing). This test fails loudly the moment the toucher set
//! changes, forcing the gen-bump invariant to be re-proven before the memo can be
//! trusted. It is intentionally independent of `Block_Stage_Gen` existing, so it
//! holds on the pre-memoize tree too.
//!
//! ```text
//! AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test parcel_8b_stage_gen_touchers
//! ```

use std::path::{Path, PathBuf};

fn emp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == ".worktrees") {
                continue;
            }
            emp_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

/// Strip a `//` line comment (there are no string literals bearing `//` in the
/// scanned corpus, so a first-occurrence cut is exact).
fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// Is `name` present in `code` as a whole identifier token (not a substring of a
/// longer identifier)? Guards against a hypothetical `Block_Stage_Keys_Foo`.
fn has_token(code: &str, name: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut from = 0;
    while let Some(rel) = code[from..].find(name) {
        let start = from + rel;
        let end = start + name.len();
        let before_ok = start == 0 || !code[..start].chars().next_back().is_some_and(is_ident);
        let after_ok = code[end..].chars().next().is_none_or(|c| !is_ident(c));
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

/// If `code` opens a proc (`proc NAME (`, optionally `pub proc`), return NAME.
fn proc_header_name(code: &str) -> Option<String> {
    let toks: Vec<&str> = code.split_whitespace().collect();
    let i = toks.iter().position(|&t| t == "proc")?;
    let name = toks.get(i + 1)?;
    // Trim a trailing `(` that abuts the name (e.g. `proc Foo(`).
    let name = name.trim_end_matches('(');
    let clean: String = name.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    (!clean.is_empty()).then_some(clean)
}

/// Attribute every whole-token occurrence of `sym` in one `.emp` file to its
/// enclosing `proc` (brace-scoped). Occurrences outside any proc are reported with
/// a `None` proc so they fail the census loudly rather than being misattributed.
fn touchers_in_file(src: &str, sym: &str) -> Vec<Option<String>> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    let mut depth: i32 = 0;
    for raw in src.lines() {
        let code = strip_comment(raw);
        if depth == 0 {
            if let Some(name) = proc_header_name(code) {
                current = Some(name);
            }
        }
        if has_token(code, sym) {
            out.push(current.clone());
        }
        depth += code.matches('{').count() as i32;
        depth -= code.matches('}').count() as i32;
        if depth <= 0 {
            depth = 0;
            current = None;
        }
    }
    out
}

#[test]
fn block_stage_keys_has_exactly_three_touchers() {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }

    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());

    const SYM: &str = "Block_Stage_Keys";
    let expected: [&str; 3] =
        ["TileCache_FindStagedBlock", "TileCache_InvalidateStaging", "TileCache_DecompressBlock"];

    // Collect (file, proc) for every occurrence across the corpus.
    let mut touchers: Vec<(String, Option<String>)> = Vec::new();
    for p in &paths {
        let src = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        for proc in touchers_in_file(&src, SYM) {
            touchers.push((p.display().to_string(), proc));
        }
    }

    // No occurrence may sit outside a proc (an unscoped write we can't audit).
    let orphans: Vec<&String> =
        touchers.iter().filter(|(_, p)| p.is_none()).map(|(f, _)| f).collect();
    assert!(orphans.is_empty(), "{SYM} referenced outside any proc in: {orphans:?}");

    // The set of touching procs must be EXACTLY the three audited ones.
    let mut got: Vec<String> = touchers.iter().filter_map(|(_, p)| p.clone()).collect();
    got.sort();
    got.dedup();
    let mut want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(
        got, want,
        "\n{SYM} toucher set changed — the memoize gen-bump audit must be redone.\n  \
         expected exactly: {want:?}\n  found:            {got:?}\n  \
         A new toucher is a staging-claim path; every claim MUST bump Block_Stage_Gen \
         or the Pfx/Cs memos become unsound."
    );
}
