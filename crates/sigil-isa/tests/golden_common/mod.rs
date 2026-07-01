//! Shared helpers for the Z80 golden-vector oracle. Included via `mod golden_common;`
//! from integration tests; not a test binary itself.
#![allow(dead_code)]

/// One parsed golden vector: the source snippet and its expected machine-code bytes.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GoldenVector {
    pub snippet: String,
    pub bytes: Vec<u8>,
}

/// Parse the golden-vector file body into `(snippet, bytes)` records.
///
/// Format, one record per line: `<snippet> => <space-separated uppercase hex>`.
/// Blank lines and lines beginning with `#` are skipped. Panics with a
/// line-numbered message on any malformed record (each Z80 form is 1..=4 bytes).
pub fn parse_golden(text: &str) -> Vec<GoldenVector> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (snippet, hex) = line
            .split_once(" => ")
            .unwrap_or_else(|| panic!("line {}: missing ' => ' separator: {line:?}", i + 1));
        let snippet = snippet.trim().to_string();
        assert!(!snippet.is_empty(), "line {}: empty snippet", i + 1);
        let bytes = hex
            .split_whitespace()
            .map(|tok| {
                u8::from_str_radix(tok, 16)
                    .unwrap_or_else(|_| panic!("line {}: bad hex byte {tok:?}", i + 1))
            })
            .collect::<Vec<u8>>();
        assert!(!bytes.is_empty(), "line {}: no bytes for {snippet:?}", i + 1);
        assert!(bytes.len() <= 4, "line {}: z80 form >4 bytes: {snippet:?}", i + 1);
        out.push(GoldenVector { snippet, bytes });
    }
    out
}

/// Count active (non-blank, non-comment) lines in a corpus/golden file body.
pub fn active_line_count(text: &str) -> usize {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .count()
}
