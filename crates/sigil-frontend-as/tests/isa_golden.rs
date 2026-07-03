//! Drive EVERY Plan-2 Z80 encoder vector through the front end (parse → lower →
//! link → flatten) and byte-match. This proves the front-end's operand classifier
//! covers the full catalog §2 ISA, not just the 9 hand-written snippets. The
//! vector file lives in sigil-isa; we read it at compile time by relative path.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::SymbolTable;

const VECTORS: &str = include_str!("../../sigil-isa/tests/z80_golden_vectors.txt");

fn assemble_one(snippet: &str) -> Vec<u8> {
    let src = format!("        cpu z80\n        phase 0\n        {snippet}\n");
    let module = assemble(&src, &Options::default())
        .unwrap_or_else(|d| panic!("assemble `{snippet}` failed: {d:?}"));
    let linked = sigil_link::link(&module.sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link `{snippet}` failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

fn parse_hex(s: &str) -> Vec<u8> {
    s.split_whitespace().map(|t| u8::from_str_radix(t, 16).unwrap()).collect()
}

#[test]
fn every_isa_vector_round_trips_through_the_frontend() {
    let mut failures = Vec::new();
    for line in VECTORS.lines() {
        let line = line.trim();
        // Match the golden file format (see sigil-isa golden_common::parse_golden):
        // blank lines and `#` comments are skipped; fields split on ` => `.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (snippet, hex) = line.split_once(" => ").expect("vector line has ` => `");
        let (snippet, want) = (snippet.trim(), parse_hex(hex.trim()));
        match std::panic::catch_unwind(|| assemble_one(snippet)) {
            Ok(got) if got == want => {}
            Ok(got) => failures.push(format!("`{snippet}`: want {want:02X?}, got {got:02X?}")),
            Err(_) => failures.push(format!("`{snippet}`: panicked")),
        }
    }
    assert!(failures.is_empty(), "front-end diverged on {} forms:\n{}", failures.len(), failures.join("\n"));
}
