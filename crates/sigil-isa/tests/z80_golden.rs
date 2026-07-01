//! CI-time validation of the committed Z80 golden-vector oracle.
//!
//! This test binary NEVER runs asl. It reads the committed
//! `tests/z80_golden_vectors.txt` (produced by the `gen-z80-vectors` generator,
//! see `src/bin/gen_z80_vectors.rs`) and checks it is well-formed and covers the
//! full catalog §2 ISA. Regenerating the golden file (re-running asl) is a
//! manual developer step:
//!
//! ```text
//! cargo run -p sigil-isa --bin gen-z80-vectors
//! ```
//!
//! The `parse_golden` helper lives in the shared `golden_common` module so the
//! Task 3/4 encoder tests can reuse it for the `encode(form) == golden[form]`
//! comparison.

mod golden_common;
use golden_common::{parse_golden, GoldenVector};

#[test]
fn parse_golden_parses_inline_sample() {
    let sample = "\
# a comment line, skipped

nop => 00
ld de,8DFCh => 11 FC 8D
bit 1,(ix+10) => DD CB 0A 4E
";
    let vectors = parse_golden(sample);
    assert_eq!(
        vectors,
        vec![
            GoldenVector { snippet: "nop".to_string(), bytes: vec![0x00] },
            GoldenVector { snippet: "ld de,8DFCh".to_string(), bytes: vec![0x11, 0xFC, 0x8D] },
            GoldenVector {
                snippet: "bit 1,(ix+10)".to_string(),
                bytes: vec![0xDD, 0xCB, 0x0A, 0x4E],
            },
        ]
    );
}

mod corpus;

#[test]
fn corpus_covers_full_isa() {
    let n = corpus::corpus().len();
    assert!(
        n >= 70,
        "corpus() must cover the full ~74-form catalog §2 ISA, found {n} entries"
    );
}

const GOLDEN: &str = include_str!("z80_golden_vectors.txt");

#[test]
fn golden_file_parses() {
    let vectors = parse_golden(GOLDEN);
    // parse_golden asserts each record is well-formed; require it is non-empty too.
    assert!(!vectors.is_empty(), "committed golden file is empty — run gen-z80-vectors");
    assert!(vectors.iter().all(|v| (1..=4).contains(&v.bytes.len())));
}

#[test]
fn golden_covers_full_isa() {
    let vectors = parse_golden(GOLDEN);
    assert!(
        vectors.len() >= 70,
        "golden file must cover the full ~74-form ISA, found {}",
        vectors.len()
    );
}

#[test]
fn golden_snippets_are_unique() {
    let vectors = parse_golden(GOLDEN);
    let mut seen = std::collections::HashSet::new();
    for v in &vectors {
        assert!(seen.insert(v.snippet.clone()), "duplicate snippet: {:?}", v.snippet);
    }
}

#[test]
fn golden_and_corpus_agree_line_for_line() {
    // Every shared-corpus() snippet must have exactly one golden vector, in order —
    // the golden is generated from corpus(), so drift here means "regenerate".
    let corpus = corpus::corpus();
    let vectors = parse_golden(GOLDEN);
    assert_eq!(
        corpus.len(),
        vectors.len(),
        "corpus() has {} snippets but golden has {} vectors — regenerate",
        corpus.len(),
        vectors.len()
    );
    for ((snip, _inst), vec) in corpus.iter().zip(&vectors) {
        assert_eq!(*snip, vec.snippet, "corpus/golden order mismatch");
    }
}

#[test]
fn known_anchor_vectors_present() {
    let vectors = parse_golden(GOLDEN);
    let find = |s: &str| vectors.iter().find(|v| v.snippet == s).map(|v| v.bytes.clone());
    // Anchors verified against real asl during authoring.
    assert_eq!(find("nop"), Some(vec![0x00]));
    assert_eq!(find("ld de,8DFCh"), Some(vec![0x11, 0xFC, 0x8D]));
    assert_eq!(find("ldir"), Some(vec![0xED, 0xB0]));
    assert_eq!(find("ex af,af'"), Some(vec![0x08]));
    assert_eq!(find("jp 1234h"), Some(vec![0xC3, 0x34, 0x12]));
    assert_eq!(find("jr $"), Some(vec![0x18, 0xFE]));
    // register-dependent ld (nn),rr — de uses ED 53, NOT the hl-only base 22
    assert_eq!(find("ld (1234h),de"), Some(vec![0xED, 0x53, 0x34, 0x12]));
    assert_eq!(find("ld (1234h),hl"), Some(vec![0x22, 0x34, 0x12]));
    assert_eq!(find("ld iy,(1234h)"), Some(vec![0xFD, 0x2A, 0x34, 0x12]));
    // DDCB/FDCB: displacement BEFORE the sub-opcode; reg field is the (HL) code 6
    assert_eq!(find("bit 1,(ix+10)"), Some(vec![0xDD, 0xCB, 0x0A, 0x4E]));
    assert_eq!(find("set 1,(iy+10)"), Some(vec![0xFD, 0xCB, 0x0A, 0xCE]));
    assert_eq!(find("neg"), Some(vec![0xED, 0x44]));
}
