//! Encoder tests: `encode(corpus form) == asl golden`, byte-for-byte. Grows one
//! per-mode test per implementation task; the final `all_forms_match_golden` gate
//! covers the entire corpus.

mod corpus_m68k;
mod m68k_common;

use m68k_common::{golden_bytes, parse_golden_m68k};
use sigil_isa::m68k::encode;

const GOLDEN: &str = include_str!("m68k_golden_vectors.txt");

/// Encode every corpus form whose snippet is in `snippets` and assert it matches golden.
fn check(snippets: &[&str]) {
    let golden = parse_golden_m68k(GOLDEN);
    let corpus = corpus_m68k::corpus_m68k();
    for snip in snippets {
        let inst = corpus
            .iter()
            .find(|(s, _)| s == snip)
            .unwrap_or_else(|| panic!("snippet {snip:?} not in corpus"))
            .1
            .clone();
        let want = golden_bytes(&golden, snip);
        let got = encode(&inst).unwrap_or_else(|e| panic!("encode {snip:?}: {e}"));
        assert_eq!(got, want, "snippet {snip:?}");
    }
}

#[test]
fn reg_direct() {
    check(&["move.w d1,d0", "move.l a1,d0"]);
}

#[test]
fn memory_indirect_source_and_dest() {
    check(&[
        "move.w (a1),d0",
        "move.w (a1)+,d0",
        "move.w -(a1),d0",
        "move.w d1,(a0)",
        "move.w d1,(a0)+",
        "move.w d1,-(a0)",
    ]);
}

#[test]
fn displacement_and_brief_extension_word() {
    check(&[
        "move.w (4,a1),d0",
        "move.w d1,(4,a0)",
        "move.w (6,a1,d2.w),d0",
        "move.l (2,a3,a4.l),d0",
    ]);
}
