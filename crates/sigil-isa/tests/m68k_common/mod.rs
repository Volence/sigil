//! Shared helpers for the 68000 golden-vector oracle. Included via `mod m68k_common;`.
//! Distinct from Z80 `golden_common` because 68000 MOVE forms exceed 4 bytes
//! (e.g. `move.w ($12345678).l,d0` is 6 bytes), so the byte-count cap differs.
#![allow(dead_code)]

/// One parsed golden vector: source snippet and expected big-endian bytes.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GoldenM68k {
    pub snippet: String,
    pub bytes: Vec<u8>,
}

/// Parse the golden file body: `<snippet> => <space-separated uppercase hex>` per line.
/// Blank lines and lines beginning with `#` are skipped. Panics (line-numbered) on any
/// malformed record. A MOVE form is 2..=10 bytes.
pub fn parse_golden_m68k(text: &str) -> Vec<GoldenM68k> {
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
        assert!(bytes.len() >= 2, "line {}: <2 bytes for {snippet:?}", i + 1);
        assert!(bytes.len() <= 10, "line {}: >10 bytes for {snippet:?}", i + 1);
        out.push(GoldenM68k { snippet, bytes });
    }
    out
}

/// Look up expected bytes by snippet, panicking if absent.
pub fn golden_bytes(golden: &[GoldenM68k], snippet: &str) -> Vec<u8> {
    golden
        .iter()
        .find(|g| g.snippet == snippet)
        .unwrap_or_else(|| panic!("no golden vector for {snippet:?}"))
        .bytes
        .clone()
}
