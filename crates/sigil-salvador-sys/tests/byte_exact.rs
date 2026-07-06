//! Byte-exactness gate: `sigil_salvador_sys::compress` must produce output
//! identical, byte for byte, to the vendored `salvador` CLI run with no
//! flags over the same input.
//!
//! The reference file `tests/vectors/sample.zx0raw` was captured by running:
//!   `aeon/tools/salvador/salvador tests/vectors/sample.bin tests/vectors/sample.zx0raw`
//! (salvador v1.4.2, no flags -> modern/V2, forward, default window).

use std::path::Path;

#[test]
fn compress_matches_reference_salvador_cli_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
    let input = std::fs::read(dir.join("sample.bin")).expect("read sample.bin fixture");
    let expected = std::fs::read(dir.join("sample.zx0raw")).expect("read sample.zx0raw fixture");

    let actual = sigil_salvador_sys::compress(&input);

    assert_eq!(
        actual.len(),
        expected.len(),
        "compressed length differs from reference salvador CLI output"
    );
    assert_eq!(
        actual, expected,
        "compressed bytes differ from reference salvador CLI output"
    );
}

#[test]
fn compress_empty_input_does_not_panic() {
    let out = sigil_salvador_sys::compress(&[]);
    // salvador's format always ends with the "last byte" marker / EOD, so the
    // empty-input stream is small but non-empty; just assert it doesn't panic
    // and returns *something* deterministic-shaped (no crash on n == 0).
    let _ = out;
}
