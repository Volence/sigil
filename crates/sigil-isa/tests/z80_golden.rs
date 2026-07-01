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
