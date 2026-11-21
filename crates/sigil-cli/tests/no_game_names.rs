//! The no-game-names gate (G0b Task 12): no crate the `sigil` driver links names a game.
//!
//! Since G0b every game on the Aeon engine is built from its `game.toml` and aeon's
//! `engine/engine.toml` (`sigil build --project`); the Rust driver that named games was
//! deleted in Task 11. This gate keeps it that way. It reads only sigil's own source and
//! the id fixture (`tests/support/game_ids.txt`), so it runs in CI with no aeon tree.
//!
//! What it scans, all derived, none hand-listed:
//! - the crates: `cargo metadata`'s NORMAL-dependency closure of `sigil-cli`, restricted
//!   to workspace members (dev- and build-dependencies never link into the driver);
//! - the files: the module tree of each crate's linked target (`sigil-cli`'s `sigil`
//!   binary, every other crate's library), followed `mod` by `mod` the way rustc does
//!   (`x.rs` / `x/mod.rs`, `#[path]`). A crate's other binaries (`src/bin/*`), its
//!   `build.rs` and its `tests/` are never reached because nothing the driver links
//!   declares them;
//! - the code: each file is parsed (`syn`), and every item, impl item, trait item and
//!   statement whose `#[cfg(..)]` REQUIRES `test` (`test`, or `all(.., test, ..)`, or an
//!   `any(..)` all of whose arms do) is removed structurally before the scan. What is
//!   left is scanned token by token: identifiers and literals. Comments are not code and
//!   are not scanned (plain comments are not tokens; doc comments, which are `#[doc]`
//!   attributes, are skipped and only counted, ungated).
//!
//! The needles (case-insensitive; a "word" is bounded by anything but an ASCII letter or
//! digit, so `demo_profile` hits and `demolish` does not):
//! - every game id in the fixture (generated from the aeon descriptions' `id` fields, plus
//!   `cherub`; `tests/game_ids_fixture.rs` holds the fixture to the descriptions);
//! - any `games.<x>` module prefix and any `games/<x>/` path, whatever `<x>` is.
//!
//! A hit is exempt only by a row in [`ALLOW`]: one file, one needle text, an exact hit
//! count, a reason and an exit. A row whose count no longer matches (in particular one that
//! matches nothing) is itself a failure: a stale exemption hides the next regression in
//! that file. The engine id (`aeon`) is counted beside the result, not gated.
//!
//! `measure_the_tree_named_by_env` runs the same verdict against another checkout (the
//! red-before-Task-11 measurement); it is `#[ignore]`d and refuses to run unnamed.

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;
use std::path::{Path, PathBuf};
use syn::visit_mut::VisitMut;

/// The binary that IS the driver. Its crate's other binaries are not the driver.
const DRIVER_PACKAGE: &str = "sigil-cli";
const DRIVER_BIN: &str = "sigil";

/// One exemption: exactly `count` hits in `file` (workspace-relative) whose token text
/// contains `needle`. The count is exact both ways: a row matching fewer hits is stale (the
/// fact moved, so the row must go), and a row matching more is a new use of the same
/// literal, which needs its own decision rather than riding an old row.
struct Allow {
    file: &'static str,
    needle: &'static str,
    count: usize,
    reason: &'static str,
    exit: &'static str,
}

/// The sound-seam facts owner ruling Q3 keeps in Rust until G0c (the sound seams' removal).
/// One row per file and needle; no directory, crate or file wildcard.
const ALLOW: &[Allow] = &[
    Allow {
        file: "crates/sigil-harness/src/seam2.rs",
        needle: "games/sonic4/map.toml",
        count: 1,
        reason: "SOUND_PLACEMENT_MAP_REL: the sound emitters' placement map; a sound-seam fact (owner Q3)",
        exit: "G0c",
    },
    Allow {
        file: "crates/sigil-harness/src/seam2.rs",
        needle: "games/sonic4/game.toml",
        count: 1,
        reason: "SOUND_GAME_DESC_REL: the sound game's description, read for the chained sound banks' \
                 [[align]] rows by Seams::validate_sound and import_check::seam_roots (owner Q3)",
        exit: "G0c",
    },
    Allow {
        file: "crates/sigil-harness/src/seam2.rs",
        needle: "games/sonic4/data/sound",
        count: 8,
        reason: "the sound data directory: six emitter joins (DAC banks, DAC/SFX bodies and heads, \
                 pitchtable, MT bank) and the two mt_bank path diagnostics that name it (owner Q3)",
        exit: "G0c",
    },
    Allow {
        file: "crates/sigil-harness/src/seam2.rs",
        needle: "games/sonic4/config/sound_ids.emp",
        count: 1,
        reason: "SOUND_IDS_REL: the song ids mt_bank.emp's drift guards read cross-seam (owner Q3)",
        exit: "G0c",
    },
    Allow {
        file: "crates/sigil-harness/src/seam1.rs",
        needle: "games/sonic4/data/sound/sfx/sfx_bank.emp",
        count: 1,
        reason: "the SFX bank whose pub consts seam 1 evaluates for the sound emit (owner Q3)",
        exit: "G0c",
    },
];

// ---------------------------------------------------------------------------------------
// The fixture.

#[derive(Debug, Clone)]
struct Ids {
    games: Vec<String>,
    engines: Vec<String>,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/game_ids.txt")
}

/// `game <id> <source>` and `engine <id> <source>` rows; `#` comments and blank lines.
fn parse_fixture(text: &str) -> Result<Ids, String> {
    let mut ids = Ids { games: vec![], engines: vec![] };
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();
        let [kind, id, _source] = cols[..] else {
            return Err(format!("fixture line {}: expected `<kind> <id> <source>`: {line}", n + 1));
        };
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') {
            return Err(format!("fixture line {}: `{id}` is not an id", n + 1));
        }
        match kind {
            "game" => ids.games.push(id.to_ascii_lowercase()),
            "engine" => ids.engines.push(id.to_ascii_lowercase()),
            other => return Err(format!("fixture line {}: unknown kind `{other}`", n + 1)),
        }
    }
    if ids.games.is_empty() {
        return Err("the fixture names no game id, so the gate would measure nothing".into());
    }
    Ok(ids)
}

fn fixture() -> Ids {
    let path = fixture_path();
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    parse_fixture(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// ---------------------------------------------------------------------------------------
// Matching.

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

/// Every needle `text` carries, as the spelling that matched (lowercased).
fn needles_in(text: &str, ids: &[String]) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let b = lower.as_bytes();
    let mut out = Vec::new();
    let bounded_at = |at: usize, len: usize| {
        (at == 0 || !is_word(b[at - 1])) && (at + len >= b.len() || !is_word(b[at + len]))
    };
    for id in ids {
        let mut from = 0;
        while let Some(off) = lower[from..].find(id.as_str()) {
            let at = from + off;
            if bounded_at(at, id.len()) {
                out.push(id.clone());
                break;
            }
            from = at + 1;
        }
    }
    // `games.<x>` and `games/<x>/`, any `<x>`.
    let seg = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'-';
    let mut from = 0;
    while let Some(off) = lower[from..].find("games") {
        let at = from + off;
        from = at + 1;
        if at > 0 && is_word(b[at - 1]) {
            continue;
        }
        let rest = &b[at + 5..];
        let Some((&sep, tail)) = rest.split_first() else { continue };
        let len = tail.iter().take_while(|&&c| seg(c)).count();
        if len == 0 {
            continue;
        }
        let x = &lower[at + 6..at + 6 + len];
        match sep {
            b'.' => out.push(format!("games.{x}")),
            b'/' if tail.get(len) == Some(&b'/') => out.push(format!("games/{x}/")),
            _ => {}
        }
    }
    out
}

// ---------------------------------------------------------------------------------------
// The structural `#[cfg(test)]` exclusion.

/// True when a `cfg` predicate cannot hold outside a test build.
fn requires_test(m: &syn::Meta) -> bool {
    match m {
        syn::Meta::Path(p) => p.is_ident("test"),
        syn::Meta::List(l) => {
            let Ok(args) = l.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
            else {
                return false;
            };
            if l.path.is_ident("all") {
                args.iter().any(requires_test)
            } else if l.path.is_ident("any") {
                !args.is_empty() && args.iter().all(requires_test)
            } else {
                false // `not(..)` and anything unknown: compiled outside tests, so scanned
            }
        }
        syn::Meta::NameValue(_) => false,
    }
}

fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("cfg") && a.parse_args::<syn::Meta>().is_ok_and(|m| requires_test(&m)))
}

struct StripTestOnly;

impl VisitMut for StripTestOnly {
    fn visit_file_mut(&mut self, f: &mut syn::File) {
        f.items.retain(|i| !item_test_only(i));
        syn::visit_mut::visit_file_mut(self, f);
    }
    fn visit_item_mod_mut(&mut self, m: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut m.content {
            items.retain(|i| !item_test_only(i));
        }
        syn::visit_mut::visit_item_mod_mut(self, m);
    }
    fn visit_item_impl_mut(&mut self, i: &mut syn::ItemImpl) {
        i.items.retain(|it| {
            !test_only(match it {
                syn::ImplItem::Const(x) => &x.attrs,
                syn::ImplItem::Fn(x) => &x.attrs,
                syn::ImplItem::Type(x) => &x.attrs,
                syn::ImplItem::Macro(x) => &x.attrs,
                _ => &[],
            })
        });
        syn::visit_mut::visit_item_impl_mut(self, i);
    }
    fn visit_item_trait_mut(&mut self, t: &mut syn::ItemTrait) {
        t.items.retain(|it| {
            !test_only(match it {
                syn::TraitItem::Const(x) => &x.attrs,
                syn::TraitItem::Fn(x) => &x.attrs,
                syn::TraitItem::Type(x) => &x.attrs,
                syn::TraitItem::Macro(x) => &x.attrs,
                _ => &[],
            })
        });
        syn::visit_mut::visit_item_trait_mut(self, t);
    }
    fn visit_block_mut(&mut self, b: &mut syn::Block) {
        b.stmts.retain(|s| match s {
            syn::Stmt::Item(i) => !item_test_only(i),
            syn::Stmt::Local(l) => !test_only(&l.attrs),
            syn::Stmt::Macro(m) => !test_only(&m.attrs),
            syn::Stmt::Expr(..) => true,
        });
        syn::visit_mut::visit_block_mut(self, b);
    }
}

fn item_attrs(i: &syn::Item) -> &[syn::Attribute] {
    use syn::Item::*;
    match i {
        Const(x) => &x.attrs,
        Enum(x) => &x.attrs,
        ExternCrate(x) => &x.attrs,
        Fn(x) => &x.attrs,
        ForeignMod(x) => &x.attrs,
        Impl(x) => &x.attrs,
        Macro(x) => &x.attrs,
        Mod(x) => &x.attrs,
        Static(x) => &x.attrs,
        Struct(x) => &x.attrs,
        Trait(x) => &x.attrs,
        TraitAlias(x) => &x.attrs,
        Type(x) => &x.attrs,
        Union(x) => &x.attrs,
        Use(x) => &x.attrs,
        _ => &[],
    }
}

fn item_test_only(i: &syn::Item) -> bool {
    test_only(item_attrs(i))
}

// ---------------------------------------------------------------------------------------
// Token scan.

#[derive(Debug, Clone)]
struct Hit {
    file: String,
    line: usize,
    text: String,
    needles: Vec<String>,
}

#[derive(Default)]
struct Counts {
    hits: Vec<(usize, String, Vec<String>)>,
    engine: usize,
    doc: usize,
    include_macro: Option<usize>,
}

/// The text a token carries: an identifier's name, a string literal's contents (escapes
/// left as written; the needles are plain ASCII), any other literal as written.
fn token_text(t: &TokenTree) -> Option<String> {
    match t {
        TokenTree::Ident(i) => Some(i.to_string()),
        TokenTree::Literal(l) => Some(l.to_string()),
        _ => None,
    }
}

fn scan_tokens(ts: TokenStream, ids: &Ids, c: &mut Counts) {
    let toks: Vec<TokenTree> = ts.into_iter().collect();
    let mut i = 0;
    while i < toks.len() {
        // `# [doc = ".."]` and `# ! [doc = ".."]`: a doc comment, not code.
        if let TokenTree::Punct(p) = &toks[i] {
            if p.as_char() == '#' {
                let mut j = i + 1;
                if matches!(toks.get(j), Some(TokenTree::Punct(q)) if q.as_char() == '!') {
                    j += 1;
                }
                if let Some(TokenTree::Group(g)) = toks.get(j) {
                    if g.delimiter() == Delimiter::Bracket {
                        let mut inner = g.stream().into_iter();
                        if matches!(inner.next(), Some(TokenTree::Ident(d)) if d == "doc") {
                            let text = g.stream().to_string();
                            if !needles_in(&text, &ids.games).is_empty() {
                                c.doc += 1;
                            }
                            i = j + 1;
                            continue;
                        }
                    }
                }
            }
        }
        match &toks[i] {
            TokenTree::Group(g) => scan_tokens(g.stream(), ids, c),
            t => {
                if let TokenTree::Ident(id) = t {
                    // `include!(..)` would splice a file this scan never opens: refuse.
                    if id == "include" && matches!(toks.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!') {
                        c.include_macro.get_or_insert(id.span().start().line);
                    }
                }
                if let Some(text) = token_text(t) {
                    let n = needles_in(&text, &ids.games);
                    if !n.is_empty() {
                        let line = match t {
                            TokenTree::Ident(x) => x.span().start().line,
                            TokenTree::Literal(x) => x.span().start().line,
                            _ => 0,
                        };
                        c.hits.push((line, text.clone(), n));
                    }
                    if !needles_in(&text, &ids.engines).is_empty() {
                        c.engine += 1;
                    }
                }
            }
        }
        i += 1;
    }
}

/// Parse one source text, strip its test-only items, scan what is left.
fn scan_source(text: &str, ids: &Ids) -> Result<(syn::File, Counts), String> {
    let mut file = syn::parse_file(text).map_err(|e| format!("parse: {e}"))?;
    StripTestOnly.visit_file_mut(&mut file);
    let mut c = Counts::default();
    scan_tokens(file.to_token_stream(), ids, &mut c);
    if let Some(line) = c.include_macro {
        return Err(format!("line {line}: `include!` splices a file this gate does not follow"));
    }
    Ok((file, c))
}

// ---------------------------------------------------------------------------------------
// The closure and the module walk.

struct Scan {
    crates: Vec<String>,
    files: usize,
    hits: Vec<Hit>,
    engine: usize,
    doc: usize,
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}

/// `(package name, linked target root)` for the driver's normal-dependency closure.
fn closure(root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let out = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--offline", "--format-version", "1", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!("cargo metadata failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("metadata json: {e}"))?;
    let packages = meta["packages"].as_array().ok_or("metadata lists no packages")?;
    let members: Vec<&str> =
        meta["workspace_members"].as_array().ok_or("no workspace_members")?.iter().filter_map(|m| m.as_str()).collect();
    let find = |name: &str| {
        packages
            .iter()
            .find(|p| p["name"].as_str() == Some(name) && p["id"].as_str().is_some_and(|id| members.contains(&id)))
    };
    let mut seen: Vec<String> = Vec::new();
    let mut queue = vec![DRIVER_PACKAGE.to_string()];
    let mut out = Vec::new();
    while let Some(name) = queue.pop() {
        if seen.contains(&name) {
            continue;
        }
        seen.push(name.clone());
        let pkg = find(&name).ok_or_else(|| format!("`{name}` is not a workspace member"))?;
        let targets = pkg["targets"].as_array().ok_or("package without targets")?;
        let kinds = |t: &serde_json::Value| -> Vec<String> {
            t["kind"].as_array().map(|k| k.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
        };
        let target = if name == DRIVER_PACKAGE {
            targets.iter().find(|t| t["name"].as_str() == Some(DRIVER_BIN) && kinds(t).iter().any(|k| k == "bin"))
        } else {
            targets.iter().find(|t| kinds(t).iter().any(|k| k.ends_with("lib") || k == "proc-macro"))
        }
        .ok_or_else(|| format!("`{name}` has no target the driver links"))?;
        let src = target["src_path"].as_str().ok_or("target without src_path")?;
        out.push((name.clone(), PathBuf::from(src)));
        for dep in pkg["dependencies"].as_array().ok_or("package without dependencies")? {
            // Normal edges only, to workspace members (a path dependency).
            if dep["kind"].is_null() && dep["path"].is_string() {
                if let Some(d) = dep["name"].as_str() {
                    queue.push(d.to_string());
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

fn path_attr(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find(|a| a.path().is_ident("path")).and_then(|a| match &a.meta {
        syn::Meta::NameValue(nv) => match &nv.value {
            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => Some(s.value()),
            _ => None,
        },
        _ => None,
    })
}

/// The files the out-of-line `mod x;` items under `items` declare. `dir` is where a
/// plain `mod x;` looks; `file_dir` is where a top-level `#[path]` resolves.
fn declared_files(items: &[syn::Item], dir: &Path, file_dir: &Path, top: bool, out: &mut Vec<(PathBuf, PathBuf)>) -> Result<(), String> {
    for item in items {
        let syn::Item::Mod(m) = item else { continue };
        let name = m.ident.to_string();
        match &m.content {
            Some((_, inner)) => {
                let sub = match path_attr(&m.attrs) {
                    Some(p) => if top { file_dir.join(p) } else { dir.join(p) },
                    None => dir.join(&name),
                };
                declared_files(inner, &sub, file_dir, false, out)?;
            }
            None => {
                let (file, child_dir) = match path_attr(&m.attrs) {
                    Some(p) => {
                        let f = if top { file_dir.join(p) } else { dir.join(p) };
                        let d = f.parent().map(Path::to_path_buf).unwrap_or_default();
                        (f, d) // a `#[path]` file nests like a `mod.rs`
                    }
                    None => {
                        let flat = dir.join(format!("{name}.rs"));
                        let nested = dir.join(&name).join("mod.rs");
                        match (flat.is_file(), nested.is_file()) {
                            (true, false) => (flat, dir.join(&name)),
                            (false, true) => (nested, dir.join(&name)),
                            (true, true) => return Err(format!("both {} and {} exist", flat.display(), nested.display())),
                            (false, false) => return Err(format!("`mod {name};` under {} names no file", dir.display())),
                        }
                    }
                };
                out.push((file, child_dir));
            }
        }
    }
    Ok(())
}

fn scan_tree(root: &Path, ids: &Ids) -> Result<Scan, String> {
    let crates = closure(root)?;
    let mut scan = Scan { crates: vec![], files: 0, hits: vec![], engine: 0, doc: 0 };
    for (name, entry) in &crates {
        scan.crates.push(name.clone());
        let entry_dir = entry.parent().ok_or("target root without a directory")?.to_path_buf();
        let mut stack = vec![(entry.clone(), entry_dir)];
        let mut visited: Vec<PathBuf> = vec![];
        while let Some((file, child_dir)) = stack.pop() {
            if visited.contains(&file) {
                continue;
            }
            visited.push(file.clone());
            let text = std::fs::read_to_string(&file).map_err(|e| format!("read {}: {e}", file.display()))?;
            let shown = rel(root, &file);
            let (parsed, c) = scan_source(&text, ids).map_err(|e| format!("{shown}: {e}"))?;
            scan.files += 1;
            scan.engine += c.engine;
            scan.doc += c.doc;
            for (line, text, needles) in c.hits {
                scan.hits.push(Hit { file: shown.clone(), line, text, needles });
            }
            let file_dir = file.parent().ok_or("file without a directory")?;
            declared_files(&parsed.items, &child_dir, file_dir, true, &mut stack)?;
        }
    }
    Ok(scan)
}

// ---------------------------------------------------------------------------------------
// The verdict.

struct Verdict {
    lines: Vec<String>,
    unexplained: usize,
    stale: usize,
}

fn verdict(root: &Path, allow: &[Allow]) -> Result<Verdict, String> {
    let ids = fixture();
    let scan = scan_tree(root, &ids)?;
    if scan.crates.is_empty() || scan.files == 0 {
        return Err(format!(
            "scanned {} files in {} crates: nothing measured, which is never a pass",
            scan.files,
            scan.crates.len()
        ));
    }
    let mut lines = vec![
        format!("game ids: {}", ids.games.join(" ")),
        format!("scanned {} files in {} crates ({})", scan.files, scan.crates.len(), scan.crates.join(" ")),
    ];
    let mut used = vec![0usize; allow.len()];
    let mut unexplained = 0;
    for h in &scan.hits {
        // The most specific row a hit matches (the longest needle), so overlapping rows in
        // one file each keep their own count.
        let row = (0..allow.len())
            .filter(|&r| allow[r].file == h.file && h.text.contains(allow[r].needle))
            .max_by_key(|&r| allow[r].needle.len());
        match row {
            Some(r) => used[r] += 1,
            None => {
                unexplained += 1;
                lines.push(format!("HIT {}:{} [{}] {}", h.file, h.line, h.needles.join(","), h.text));
            }
        }
    }
    let mut stale = 0;
    for (a, n) in allow.iter().zip(&used) {
        if *n != a.count {
            stale += 1;
            let what = if *n == 0 { "matches nothing".to_string() } else { format!("matches {n} hits, not {}", a.count) };
            lines.push(format!("STALE allow row {} `{}` {what} ({}; exit {})", a.file, a.needle, a.reason, a.exit));
        }
    }
    lines.push(format!(
        "{} game-name hits: {} unexplained, {} exempt by an allow-list of {} rows ({} stale)",
        scan.hits.len(),
        unexplained,
        used.iter().sum::<usize>(),
        allow.len(),
        stale
    ));
    lines.push(format!(
        "engine id ({}): {} code hits, not gated; doc-comment game-id hits: {}, not gated",
        ids.engines.join(" "),
        scan.engine,
        scan.doc
    ));
    Ok(Verdict { lines, unexplained, stale })
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("workspace root")
}

fn judge(root: &Path) {
    let v = verdict(root, ALLOW).unwrap_or_else(|e| panic!("the no-game-names gate could not measure: {e}"));
    for l in &v.lines {
        println!("{l}");
    }
    assert!(
        v.unexplained == 0 && v.stale == 0,
        "a crate the driver links names a game ({} hits) or an exemption is stale ({} rows):\n{}",
        v.unexplained,
        v.stale,
        v.lines.join("\n")
    );
}

#[test]
fn no_crate_the_driver_links_names_a_game() {
    judge(&workspace_root());
}

/// The same verdict against another checkout, named by `SIGIL_NO_GAME_NAMES_ROOT`.
#[test]
#[ignore = "measures another checkout; run with SIGIL_NO_GAME_NAMES_ROOT=<workspace> -- --ignored"]
fn measure_the_tree_named_by_env() {
    let root = std::env::var("SIGIL_NO_GAME_NAMES_ROOT").expect("SIGIL_NO_GAME_NAMES_ROOT names no checkout");
    judge(Path::new(&root));
}

#[test]
fn every_allow_row_names_a_reason_and_an_exit() {
    for a in ALLOW {
        assert!(!a.file.is_empty() && !a.needle.is_empty() && !a.reason.is_empty(), "blank allow row {}", a.file);
        assert!(a.file.ends_with(".rs") && !a.file.contains('*'), "allow row {} is not one file", a.file);
        assert!(a.count > 0, "allow row {} `{}` exempts nothing", a.file, a.needle);
        assert_eq!(a.exit, "G0c", "allow row {} `{}` has no G0c exit", a.file, a.needle);
    }
    let mut keys: Vec<(&str, &str)> = ALLOW.iter().map(|a| (a.file, a.needle)).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), ALLOW.len(), "two allow rows share a file and needle");
}

// ---------------------------------------------------------------------------------------
// The scanner's own rules, on synthetic sources.

fn ids_for_tests() -> Ids {
    Ids { games: vec!["sonic4".into(), "demo".into(), "cherub".into()], engines: vec!["aeon".into()] }
}

fn hits(src: &str) -> Vec<String> {
    let (_, c) = scan_source(src, &ids_for_tests()).expect("parses");
    c.hits.into_iter().map(|(l, t, _)| format!("{l}:{t}")).collect()
}

#[test]
fn a_needle_inside_a_cfg_test_mod_is_not_a_hit() {
    let src = "fn a() {}\n#[cfg(test)]\nmod tests {\n    const P: &str = \"games/sonic4/x\";\n    fn demo() {}\n}\n";
    assert!(hits(src).is_empty(), "{:?}", hits(src));
}

#[test]
fn a_needle_after_a_cfg_test_mod_is_a_hit() {
    let src = "#[cfg(test)]\nmod tests {\n    fn x() {}\n}\nconst P: &str = \"games/demo/\";\n";
    assert_eq!(hits(src), vec!["5:\"games/demo/\"".to_string()]);
}

#[test]
fn the_cfg_predicate_is_evaluated_not_matched() {
    let src = "#[cfg(all(test, unix))] const A: &str = \"sonic4\";\n\
               #[cfg(any(test, feature = \"x\"))] const B: &str = \"sonic4\";\n\
               #[cfg(not(test))] const C: &str = \"sonic4\";\n\
               #[cfg(any(test, all(test, unix)))] const D: &str = \"sonic4\";\n";
    assert_eq!(hits(src), vec!["2:\"sonic4\"".to_string(), "3:\"sonic4\"".to_string()]);
}

#[test]
fn test_only_impl_items_and_statements_are_removed() {
    let src = "struct S;\nimpl S {\n    #[cfg(test)]\n    fn demo() {}\n    fn keep() {\n        #[cfg(test)]\n        let _x = \"sonic4\";\n        let _y = \"cherub\";\n    }\n}\n";
    assert_eq!(hits(src), vec!["8:\"cherub\"".to_string()]);
}

#[test]
fn comments_are_not_code() {
    let src = "/// games/sonic4/x\n// sonic4\n/* demo */\nfn f() {}\n";
    let (_, c) = scan_source(src, &ids_for_tests()).unwrap();
    assert!(c.hits.is_empty());
    assert_eq!(c.doc, 1, "the doc comment is counted, not gated");
}

#[test]
fn the_needles_are_words_and_any_games_segment() {
    let ids = ids_for_tests().games;
    assert_eq!(needles_in("demo_profile", &ids), vec!["demo"]);
    assert_eq!(needles_in("SONIC4_ROM", &ids), vec!["sonic4"]);
    assert!(needles_in("demolish", &ids).is_empty());
    assert!(needles_in("sonic42", &ids).is_empty());
    assert_eq!(needles_in("games.zeta.root", &ids), vec!["games.zeta"]);
    assert_eq!(needles_in("\"games/zeta/game.toml\"", &ids), vec!["games/zeta/"]);
    assert!(needles_in("\"games/{}/game.toml\"", &ids).is_empty(), "a template names no game");
    assert!(needles_in("\"games/\"", &ids).is_empty());
    assert!(needles_in("mygames.x", &ids).is_empty());
}

#[test]
fn an_include_macro_is_refused() {
    assert!(scan_source("include!(\"x.rs\");\n", &ids_for_tests()).is_err());
}

#[test]
fn the_fixture_parser_refuses_what_it_cannot_read() {
    assert!(parse_fixture("# nothing\n").is_err(), "no game id = unmeasurable");
    assert!(parse_fixture("game sonic4\n").is_err());
    assert!(parse_fixture("gam sonic4 x\n").is_err());
    assert!(parse_fixture("game so*nic x\n").is_err());
    assert_eq!(parse_fixture("game Sonic4 x\nengine aeon y\n").unwrap().games, vec!["sonic4"]);
}
