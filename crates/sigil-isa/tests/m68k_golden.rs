//! CI-time validation of the committed 68000 golden-vector oracle. Never runs asl.

mod m68k_common;
use m68k_common::parse_golden_m68k;

mod corpus_m68k;

const GOLDEN: &str = include_str!("m68k_golden_vectors.txt");

#[test]
fn golden_file_parses_and_is_nonempty() {
    let vectors = parse_golden_m68k(GOLDEN);
    assert!(!vectors.is_empty(), "committed golden file is empty — run gen-m68k-vectors");
}

#[test]
fn golden_covers_the_full_corpus() {
    let vectors = parse_golden_m68k(GOLDEN);
    let corpus = corpus_m68k::corpus_m68k();
    assert_eq!(
        vectors.len(),
        corpus.len(),
        "golden vector count ({}) != corpus count ({}) — regenerate",
        vectors.len(),
        corpus.len()
    );
    for (snippet, _) in &corpus {
        assert!(
            vectors.iter().any(|v| v.snippet == *snippet),
            "corpus snippet {snippet:?} missing from golden — regenerate"
        );
    }
}

#[test]
fn golden_snippets_are_unique() {
    let vectors = parse_golden_m68k(GOLDEN);
    let mut seen = std::collections::BTreeSet::new();
    for v in &vectors {
        assert!(seen.insert(v.snippet.clone()), "duplicate golden snippet: {:?}", v.snippet);
    }
}
