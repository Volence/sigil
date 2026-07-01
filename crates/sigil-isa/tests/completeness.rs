//! Completeness gate: every asl-verified golden form must map to an `Instruction`
//! (via the shared canonical `corpus()`) that `encode()`s to the exact golden bytes —
//! proving no catalog form is uncovered and no corpus form is orphaned.

use std::collections::{HashMap, HashSet};

use sigil_isa::z80::{encode, Instruction};

// The single source of truth: the SAME `corpus()` Task 2 generates its golden from.
mod corpus;

/// The committed asl golden vectors (produced by the Plan-2 oracle generator).
const GOLDEN: &str = include_str!("z80_golden_vectors.txt");

/// Normalize a snippet to a stable key: lowercase, with runs of internal
/// whitespace collapsed to a single space (leading/trailing trimmed).
/// Defends the golden↔corpus match against incidental spacing/case differences
/// while preserving mnemonic-significant spaces — e.g. `rrca` (0F) stays distinct
/// from `rrc a` (CB 0F), both of which the canonical corpus carries.
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Parse the golden file into `(snippet, bytes)` records.
fn parse_golden() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for line in GOLDEN.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (snip, hex) = line
            .split_once(" => ")
            .unwrap_or_else(|| panic!("golden line not <snippet> => <hex>: {line:?}"));
        let bytes = hex
            .split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap_or_else(|_| panic!("bad hex byte {b:?}")))
            .collect();
        out.push((snip.to_string(), bytes));
    }
    out
}

#[test]
fn every_golden_form_is_encoded() {
    let golden = parse_golden();
    assert!(
        golden.len() >= 74,
        "golden file has {} forms; expected the full catalog (>= 74)",
        golden.len()
    );

    // Build the snippet→Instruction map from the shared corpus, keyed by normalized snippet.
    let mut map: HashMap<String, Instruction> = HashMap::new();
    for (snip, i) in corpus::corpus() {
        assert!(
            map.insert(norm(snip), i).is_none(),
            "duplicate corpus snippet: {snip}"
        );
    }

    // (1) Every asl-verified golden form maps to an Instruction encoding to its bytes.
    let mut uncovered = Vec::new();
    for (snip, bytes) in &golden {
        match map.get(&norm(snip)) {
            None => uncovered.push(snip.clone()),
            Some(i) => {
                let got =
                    encode(i).unwrap_or_else(|e| panic!("encode failed for `{snip}`: {e:?}"));
                assert_eq!(&got, bytes, "byte mismatch for `{snip}`");
            }
        }
    }
    assert!(uncovered.is_empty(), "uncovered golden form(s): {uncovered:?}");

    // (2) No stale/orphan mappings: every corpus form appears in the golden file.
    let golden_keys: HashSet<String> = golden.iter().map(|(s, _)| norm(s)).collect();
    let orphans: Vec<&String> = map.keys().filter(|k| !golden_keys.contains(*k)).collect();
    assert!(
        orphans.is_empty(),
        "corpus snippet(s) not present in golden file: {orphans:?}"
    );
}
