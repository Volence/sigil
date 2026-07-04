//! End-to-end gate: assemble each committed snippet through the front end
//! (→ link → flatten) and compare to golden bytes. Golden bytes for these
//! snippets are hand-verified; a manual `gen_snippet_vectors` bin (added
//! separately) can regenerate them from real `asl`.

use sigil_frontend_as::{assemble, Options};
use sigil_ir::SymbolTable;

fn assemble_bytes(asm: &str) -> Vec<u8> {
    let module = assemble(asm, &Options::default()).expect("assemble");
    // `resolve_layout` picks the jmp/jsr abs.w/abs.l width and lowers every
    // `Fragment::JmpJsrSym` to a concrete `Data` fragment before `link()` runs
    // (see `sigil-harness/tests/m1b_gate.rs` for the same composition). Over a
    // module with no `JmpJsrSym` fragments (every snippet before T5c) this is a
    // no-op, so wiring it in here must not perturb any existing golden bytes.
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

fn blocks() -> Vec<(String, String, Vec<u8>)> {
    let text = include_str!("snippets_golden.txt");
    let mut out = Vec::new();
    let mut name = String::new();
    let mut asm = String::new();
    let mut hex = String::new();
    let mut in_bytes = false;
    for line in text.lines() {
        if let Some(n) = line
            .strip_prefix("=== ")
            .and_then(|s| s.strip_suffix(" ==="))
        {
            if !name.is_empty() {
                out.push((name.clone(), asm.clone(), parse_hex(&hex)));
            }
            name = n.to_string();
            asm.clear();
            hex.clear();
            in_bytes = false;
        } else if line.trim() == "--- bytes ---" {
            in_bytes = true;
        } else if in_bytes {
            hex.push_str(line);
            hex.push(' ');
        } else {
            asm.push_str(line);
            asm.push('\n');
        }
    }
    if !name.is_empty() {
        out.push((name, asm, parse_hex(&hex)));
    }
    out
}

fn parse_hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|t| u8::from_str_radix(t, 16).unwrap())
        .collect()
}

#[test]
fn snippets_match_golden() {
    for (name, asm, want) in blocks() {
        assert_eq!(
            assemble_bytes(&asm),
            want,
            "snippet `{name}` diverged from golden"
        );
    }
}
