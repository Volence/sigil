//! The vendored `empyrean.build` contract (`contract/build/`) must be exactly what its
//! `SOURCE` file says it is. This test runs in sigil CI with no other repo present: it
//! recomputes every file's SHA-256 and compares with the pinned list, and it fails on a
//! file that is on disk but unlisted or listed but missing. Whether the pinned list still
//! equals empyrean's tree is `scripts/check-vendored-contract.sh`'s question (it needs
//! `EMPYREAN_DIR`), not this test's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sigil_link::export::{sha256_hex, sha256_of};

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contract/build")
}

fn walk(dir: &Path, base: &Path, out: &mut BTreeMap<String, String>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            walk(&p, base, out);
        } else {
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
            if rel != "SOURCE" {
                out.insert(rel, sha256_hex(&sha256_of(&std::fs::read(&p).unwrap())));
            }
        }
    }
}

/// The `<sha256>  <path>` lines of SOURCE (the lines that are not `key: value` or comments).
fn pinned(source: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for l in source.lines() {
        let Some((h, p)) = l.split_once("  ") else { continue };
        if h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
            assert!(m.insert(p.to_string(), h.to_string()).is_none(), "SOURCE lists {p} twice");
        }
    }
    m
}

#[test]
fn vendored_schema_matches_source() {
    let dir = contract_dir();
    let source = std::fs::read_to_string(dir.join("SOURCE")).expect("contract/build/SOURCE must exist");
    assert!(
        source.lines().any(|l| l.strip_prefix("empyrean-commit: ").is_some_and(|c| c.len() == 40)),
        "SOURCE names no 40-digit empyrean-commit"
    );
    let want = pinned(&source);
    assert!(!want.is_empty(), "SOURCE pins no files: nothing would be checked");
    for needed in ["empyrean.build-1.schema.json", "validate_build_schema.py"] {
        assert!(want.contains_key(needed), "SOURCE does not pin {needed}");
    }
    assert!(want.keys().any(|k| k.starts_with("vectors/pass/")) && want.keys().any(|k| k.starts_with("vectors/red/")));
    let mut have = BTreeMap::new();
    walk(&dir, &dir, &mut have);
    assert_eq!(have, want, "contract/build/ differs from SOURCE: re-vendor from empyrean, do not edit here");
}

#[test]
fn pinned_parser_reads_only_hash_lines() {
    let h = "a".repeat(64);
    let m = pinned(&format!("empyrean-commit: x\nfiles:\n{h}  a/b.json\n{}  c.json\n", "G".repeat(64)));
    assert_eq!(m.len(), 1);
    assert_eq!(m["a/b.json"], h);
}
