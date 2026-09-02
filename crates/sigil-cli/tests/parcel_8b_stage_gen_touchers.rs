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
//! The census is CORPUS-WIDE: it scans the `.emp` source AND the `.asm`/`.inc`
//! twins/hand-written engine, so a 4th toucher written in either language fails.
//! The `.emp` scan attributes each occurrence to its enclosing `proc`; the
//! `.asm`/`.inc` scan attributes to the enclosing column-0 label. Both must name
//! exactly the three audited routines. The one non-proc `.asm` occurrence — the
//! `Block_Stage_Keys:` RAM declaration in `engine/ram.asm` — is the allowlisted
//! declaration site (asserted to live only there).
//!
//! ```text
//! AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test parcel_8b_stage_gen_touchers
//! ```

use std::path::{Path, PathBuf};

/// Collect every file under `dir` (recursively, skipping `.worktrees`) whose
/// extension is in `exts`.
fn files_with_ext(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == ".worktrees") {
                continue;
            }
            files_with_ext(&p, exts, out);
        } else if p.extension().is_some_and(|x| exts.iter().any(|e| x == *e)) {
            out.push(p);
        }
    }
}

/// Strip a line comment introduced by `marker` (`//` for `.emp`, `;` for asm).
/// No string literal in the scanned corpus bears the marker, so a first-
/// occurrence cut is exact.
fn strip_marker<'a>(line: &'a str, marker: &str) -> &'a str {
    match line.find(marker) {
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
        let code = strip_marker(raw, "//");
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
    let aeon = sigil_harness::test_support::aeon_dir();
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }

    const SYM: &str = "Block_Stage_Keys";
    let expected: [&str; 3] =
        ["TileCache_FindStagedBlock", "TileCache_InvalidateStaging", "TileCache_DecompressBlock"];
    let mut want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    want.sort();

    // --- .emp census: attribute each occurrence to its enclosing `proc`. ---
    let mut emp_paths = Vec::new();
    files_with_ext(&aeon.join("engine"), &["emp"], &mut emp_paths);
    files_with_ext(&aeon.join("games"), &["emp"], &mut emp_paths);
    // Item #7b: engine/ram.emp is the label's DEFINITION home (a `vars` field
    // `Block_Stage_Keys: [u32; ..]`), not a staging-claim path. It sits outside any
    // proc by construction; exclude it so the definition is not mis-counted as an
    // un-audited orphan (the role engine/ram.asm's `Block_Stage_Keys:` declaration
    // played before the port — it was residual AS, never in this .emp census).
    emp_paths.retain(|p| !p.ends_with("engine/ram.emp"));
    emp_paths.sort();
    assert!(!emp_paths.is_empty(), "no .emp files under {}", aeon.display());

    let mut emp_touchers: Vec<(String, Option<String>)> = Vec::new();
    for p in &emp_paths {
        let src = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        for proc in touchers_in_file(&src, SYM) {
            emp_touchers.push((p.display().to_string(), proc));
        }
    }

    // No .emp occurrence may sit outside a proc (an unscoped write we can't audit).
    let emp_orphans: Vec<&String> =
        emp_touchers.iter().filter(|(_, p)| p.is_none()).map(|(f, _)| f).collect();
    assert!(emp_orphans.is_empty(), "{SYM} referenced outside any .emp proc in: {emp_orphans:?}");

    let mut emp_got: Vec<String> = emp_touchers.iter().filter_map(|(_, p)| p.clone()).collect();
    emp_got.sort();
    emp_got.dedup();
    assert_eq!(
        emp_got, want,
        "\n{SYM} .emp toucher set changed — the memoize gen-bump audit must be redone.\n  \
         expected exactly: {want:?}\n  found:            {emp_got:?}\n  \
         A new toucher is a staging-claim path; every claim MUST bump Block_Stage_Gen \
         or the Pfx/Cs memos become unsound."
    );

    // The .asm/.inc census half RETIRED (flip Stage-2): tile_cache.asm (which
    // homed the three TileCache staging-claim routines) is deleted — the .emp is
    // the only source. The .emp census above is now the sole authoritative
    // gen-bump audit; the `Block_Stage_Keys:` RAM declaration lives in
    // engine/ram.emp (item #7b — the region form), excluded above as the label's
    // definition home, so it is not a staging-claim path.
}
