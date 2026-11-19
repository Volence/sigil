//! The G0b descriptions in the aeon REFERENCE tree load, and say what the Rust driver says.
//!
//! Reference-dependent: the tree is `AEON_DIR` (or `EMPYREAN_SUITE_ROOT/aeon`), resolved by
//! `sigil_build::reference::aeon_dir`, a mirror of sigil-harness's A8 / d-18 resolver (this
//! crate may not depend on sigil-harness). A missing description skips with the SKIP marker,
//! and is a failure under `SIGIL_STRICT_GATE=1`. A G0b landing run names a copy of the aeon
//! `feat/g0b-descriptions` tip, which carries the files.
//!
//! Every expectation is read out of the sigil-harness Rust source as text, never retyped and
//! never linked. Task 3's agree-with-Rust test does the field-by-field comparison; these
//! prove the loaders read real content rather than defaults.

use sigil_build::describe::{load_baseline, load_engine, load_game, resolve_shape};
use std::path::{Path, PathBuf};

fn strict_gate() -> bool {
    std::env::var_os("SIGIL_STRICT_GATE").is_some()
}

/// The guard every test here opens with: `Some(aeon)` when the reference tree carries every
/// path in `rels`, `None` (skip) when it does not, a panic naming the path under
/// `SIGIL_STRICT_GATE=1`. Mirrors sigil-harness `test_support::reference_tree`.
fn reference_tree(rels: &[&str]) -> Option<PathBuf> {
    let aeon = sigil_build::reference::aeon_dir();
    if let Some(missing) = rels.iter().find(|rel| !aeon.join(rel).exists()) {
        let path = aeon.join(missing);
        assert!(!strict_gate(), "SIGIL_STRICT_GATE set but reference missing: {}", path.display());
        eprintln!("skip: reference not at {} (set AEON_DIR)", path.display());
        return None;
    }
    Some(aeon)
}

// ── expectations read out of the Rust driver's source text (this crate may not depend on
// sigil-harness, so the constants are read as text, never retyped here) ──

fn harness_src(file: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/src").join(file);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read the Rust authority {}: {e}", p.display()))
}

/// The string literals on one line (comments stripped).
fn literals(line: &str) -> Vec<String> {
    let code = line.split("//").next().unwrap_or("");
    code.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

/// The rows of the slice constant whose declaration line contains `decl`, one entry per
/// non-comment line, each the string literals on that line. Loud when absent or empty.
fn rust_rows(src: &str, decl: &str) -> Vec<Vec<String>> {
    let mut lines = src.lines().skip_while(|l| !l.contains(decl));
    let head = lines.next().unwrap_or_else(|| panic!("Rust authority lost `{decl}`"));
    if let Some(body) = head.split_once("= &[").map(|(_, b)| b).filter(|b| b.contains("];")) {
        // A one-line slice: its rows are the literals between the brackets.
        let rows = vec![literals(body)];
        assert!(!rows[0].is_empty(), "`{decl}` read as empty, which is unmeasured, not empty");
        return rows;
    }
    let rows: Vec<Vec<String>> = lines
        .take_while(|l| !l.trim_start().starts_with("];"))
        .filter(|l| !l.trim_start().starts_with("//"))
        .map(literals)
        .filter(|v| !v.is_empty())
        .collect();
    assert!(!rows.is_empty(), "`{decl}` read as empty, which is unmeasured, not empty");
    rows
}

fn rust_list(src: &str, decl: &str) -> Vec<String> {
    rust_rows(src, decl).into_iter().flatten().collect()
}

/// `name`'s value in the `emp_defines` of the profile fn `func`.
fn rust_profile_define(func: &str, name: &str) -> i64 {
    let src = harness_src("native.rs");
    let body = src.split(&format!("pub fn {func}(")).nth(1).unwrap_or_else(|| panic!("no `{func}`"));
    let body = body.split("\npub fn ").next().unwrap();
    let line = body
        .lines()
        .find(|l| l.contains(&format!("(\"{name}\", ")))
        .unwrap_or_else(|| panic!("`{func}` lost define `{name}`"));
    let v = line.split(&format!("(\"{name}\", ")).nth(1).unwrap().split(')').next().unwrap().trim();
    match v.strip_prefix("0x") {
        Some(h) => i64::from_str_radix(h, 16).unwrap(),
        None => v.parse().unwrap(),
    }
}

/// How many shapes the Rust profile constructors make for `game` ("sonic4" or "demo"):
/// every `pub fn *_profile(..) -> GameProfile`, the demo one for demo and the rest for
/// sonic4, a `debug: bool` constructor counting twice.
fn rust_shape_count(game: &str) -> usize {
    harness_src("native.rs")
        .lines()
        .filter(|l| l.starts_with("pub fn ") && l.contains("_profile(") && l.contains("-> GameProfile"))
        .filter(|l| (game == "demo") == l.starts_with("pub fn demo_profile("))
        .map(|l| if l.contains("debug: bool") { 2 } else { 1 })
        .sum()
}

/// The `section_align.rs` rows stronger than the 68000 word rule: `(label, required)`.
fn rust_align_exceptions() -> Vec<(String, u32)> {
    let src = harness_src("section_align.rs");
    let rows: Vec<(String, u32)> = src
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("d(\""))
        .filter_map(|rest| {
            let (label, tail) = rest.split_once("\", ")?;
            let n = tail.split(',').next()?.trim();
            let v = match n.strip_prefix("0x") {
                Some(h) => u32::from_str_radix(h, 16).ok()?,
                None => n.parse().ok()?,
            };
            Some((label.to_string(), v))
        })
        .collect();
    assert!(rows.len() > 6, "section_align.rs DECLARED read as {} rows, which is unmeasured", rows.len());
    rows.into_iter().filter(|(_, v)| *v != 2).collect()
}

#[test]
fn sonic4_game_toml_loads() {
    let Some(tree) = reference_tree(&["games/sonic4/game.toml", "games/demo/game.toml", "engine/engine.toml"]) else {
        return;
    };
    let s4 = load_game(&tree.join("games/sonic4/game.toml")).unwrap_or_else(|e| panic!("{e}"));
    let demo = load_game(&tree.join("games/demo/game.toml")).unwrap_or_else(|e| panic!("{e}"));
    let engine = load_engine(&tree.join("engine/engine.toml")).unwrap_or_else(|e| panic!("{e}"));

    // Real content, each expectation read from the Rust authority.
    assert_eq!(s4.shapes.len(), rust_shape_count("sonic4"), "sonic4 shape count");
    assert_eq!(demo.shapes.len(), rust_shape_count("demo"), "demo shape count");
    assert_eq!(s4.defines["MAX_RING_BUFFER"], rust_profile_define("sonic4_profile", "MAX_RING_BUFFER"));
    assert_eq!(demo.defines["MAX_RING_BUFFER"], rust_profile_define("demo_profile", "MAX_RING_BUFFER"));
    assert_ne!(s4.defines["MAX_RING_BUFFER"], demo.defines["MAX_RING_BUFFER"], "the two games differ here");

    // The six alignment exceptions: the engine's and sonic4's rows together, label and
    // quantum, against section_align.rs's rows stronger than the word rule.
    let mut got: Vec<(String, u32)> =
        engine.align.iter().chain(&s4.align).map(|r| (r.label.clone(), r.align)).collect();
    got.sort();
    let mut want = rust_align_exceptions();
    want.sort();
    assert_eq!(got, want, "align exceptions");
    assert!(demo.align.is_empty(), "demo carries none of the sound banks");

    // Every shape flattens, and the fixtures are exactly the three ruled ones.
    for g in [&s4, &demo] {
        for name in g.shapes.keys() {
            resolve_shape(g, name).unwrap_or_else(|e| panic!("{}: {e}", g.id));
        }
    }
    let fixtures: Vec<&str> =
        s4.shapes.keys().filter(|n| resolve_shape(&s4, n).unwrap().fixture).map(String::as_str).collect();
    assert_eq!(fixtures, ["config-a-keystones-flipped", "stress-art", "stress-evict"]);

    // The baselines each description names load, and sonic4's game rows plus the
    // engine's are exactly the Rust shape-invariant D1c table (a multiset).
    let eb = load_baseline(&tree.join("engine").join(&engine.baseline)).unwrap_or_else(|e| panic!("{e}"));
    let sb = load_baseline(&tree.join("games/sonic4").join(&s4.baseline)).unwrap_or_else(|e| panic!("{e}"));
    let db = load_baseline(&tree.join("games/demo").join(&demo.baseline)).unwrap_or_else(|e| panic!("{e}"));
    let cb = harness_src("contract_baseline.rs");
    let triples = |decl: &str| -> Vec<(String, String, String)> {
        let mut v: Vec<_> = rust_rows(&cb, decl).into_iter().map(|r| (r[0].clone(), r[1].clone(), r[2].clone())).collect();
        v.sort();
        v
    };
    let mut d1c: Vec<_> = eb.d1c.iter().chain(&sb.d1c).cloned().collect();
    d1c.sort();
    assert_eq!(d1c, triples("pub const D1C_BASELINE"));
    let mut dbg = eb.d1c_debug.clone();
    dbg.sort();
    assert_eq!(dbg, triples("pub const D1C_DEBUG_EXTRA"));
    let mut demo_rows = db.d1c.clone();
    demo_rows.sort();
    assert_eq!(demo_rows, triples("pub const D1C_DEMO_EXTRA"));
    let mut z80 = eb.z80_out_unverified.clone();
    z80.sort();
    let mut want_z80: Vec<_> = rust_rows(&cb, "pub const Z80_OUT_UNVERIFIED_BASELINE")
        .into_iter()
        .map(|r| (r[0].clone(), r[1].clone()))
        .collect();
    want_z80.sort();
    assert_eq!(z80, want_z80);
}

#[test]
fn engine_toml_loads() {
    let Some(tree) = reference_tree(&["engine/engine.toml"]) else {
        return;
    };
    let e = load_engine(&tree.join("engine/engine.toml")).unwrap_or_else(|e| panic!("{e}"));
    let native = harness_src("native.rs");
    // Order matters for the helpers, so the whole list, not only its length.
    assert_eq!(e.helpers, rust_list(&native, "const COMPTIME_HELPERS: &[&str]"));
    assert_eq!(e.ram_helpers, rust_list(&native, "const RAM_HELPERS: &[&str]"));
    let twins: Vec<(String, String, String)> = rust_rows(&native, "const STRUCT_OFFSET_TWINS")
        .into_iter()
        .map(|r| (r[0].trim_start_matches("engine/").to_string(), r[1].clone(), r[2].clone()))
        .collect();
    assert_eq!(e.struct_twins, twins, "struct twins (files relative to the engine root)");
    let sb = harness_src("sound_bank_ids.rs");
    let names: Vec<String> = e.sound.blob_bank_ids.iter().map(|b| b.name.clone()).collect();
    assert_eq!(names, rust_list(&sb, "pub const BLOB_BANK_ID_CONSTS"));
    let mr = harness_src("module_roots.rs");
    let str_const = |src: &str, decl: &str| -> String {
        let line = src.lines().find(|l| l.contains(decl)).unwrap_or_else(|| panic!("Rust authority lost `{decl}`"));
        literals(line).into_iter().next().unwrap_or_else(|| panic!("`{decl}` has no literal"))
    };
    assert_eq!(e.terminus, str_const(&mr, "pub const ENGINE_TERMINUS"));
    assert_eq!(e.error_handler, str_const(&mr, "pub const ERROR_HANDLER_MODULE_ID"));
    assert_eq!(e.release_fault, str_const(&mr, "pub const RELEASE_FAULT_MODULE_ID"));
    // The blob label moved into this crate with the appendix (G0b Task 4), so it is
    // linked, not read as harness text.
    assert_eq!(e.error_handler_blob, sigil_build::appendix::ERROR_HANDLER_BLOB_LABEL);
    // G0b Task 5: the appendix's other two engine facts, which the new driver reads from
    // here and the old one keeps as literals until the cutover.
    assert_eq!(e.error_handler_blob_len, sigil_build::appendix::ERROR_HANDLER_BLOB_LEN);
    let engine_root = std::path::Path::new("engine");
    let convsym: std::path::PathBuf = engine_root.join(&e.convsym).components().fold(std::path::PathBuf::new(), |mut p, c| {
        match c {
            std::path::Component::ParentDir => {
                p.pop();
            }
            c => p.push(c),
        }
        p
    });
    assert_eq!(convsym, std::path::Path::new("tools/convsym"), "engine.toml `convsym` must name the checkout's tools/convsym (appendix.rs)");
    assert!(!e.gate.is_empty() && e.required_labels.contains(&"EndOfRom".to_string()));
}
