//! `gen-snippet-vectors` — regenerate the committed snippet golden bytes from asl.
//!
//! MANUAL developer tool — NOT run in CI. It reads the `asm` blocks from
//! `tests/snippets_golden.txt`, assembles each one with the real `asl` (asl 1.42)
//! from the Aeon tree (`-cpu 68000 -q -L -U`), extracts the emitted bytes with
//! `p2bin`, and rewrites each block's `--- bytes ---` section in place. The CI
//! test (`tests/asl_snippets.rs`) reads the committed golden bytes instead and
//! never needs asl.
//!
//! ```text
//! cargo run -p sigil-frontend-as --bin gen-snippet-vectors
//! AEON_DIR=/path/to/aeon cargo run -p sigil-frontend-as --bin gen-snippet-vectors
//! ```
//!
//! The committed golden bytes are **generator-produced from real asl** and
//! regenerate byte-identically (running this tool on the committed file is a
//! git-clean no-op — the non-circularity invariant: each new snippet block must
//! churn ONLY its own bytes, proving every committed golden is authentic asl
//! output, not a value the implementation happened to emit).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One parsed snippet block: its name and its assembly source lines.
struct Block {
    name: String,
    asm: String,
}

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let golden_path = manifest.join("tests/snippets_golden.txt");
    let text = fs::read_to_string(&golden_path).expect("read snippets_golden.txt");
    let blocks = parse_blocks(&text);

    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let aeon = PathBuf::from(aeon);
    let asl = aeon.join("tools/asl");
    let p2bin = aeon.join("tools/p2bin");
    assert!(
        asl.is_file(),
        "asl not found at {} (set AEON_DIR)",
        asl.display()
    );
    assert!(
        p2bin.is_file(),
        "p2bin not found at {} (set AEON_DIR)",
        p2bin.display()
    );

    let work = std::env::temp_dir().join("sigil_snippet_gen");
    fs::create_dir_all(&work).expect("create work dir");

    let mut out = String::new();
    for b in &blocks {
        let bytes = assemble(&aeon, &asl, &p2bin, &work, &b.asm);
        let hex = bytes
            .iter()
            .map(|x| format!("{x:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str("=== ");
        out.push_str(&b.name);
        out.push_str(" ===\n");
        out.push_str(&b.asm);
        out.push_str("--- bytes ---\n");
        out.push_str(&hex);
        out.push('\n');
    }

    fs::write(&golden_path, &out).expect("write golden file");
    eprintln!(
        "wrote {} snippet vectors to {}",
        blocks.len(),
        golden_path.display()
    );
}

/// Parse the `=== name ===` / `--- bytes ---` block file into (name, asm) pairs,
/// dropping the existing golden byte lines (this tool regenerates them).
fn parse_blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut name = String::new();
    let mut asm = String::new();
    let mut in_bytes = false;
    for line in text.lines() {
        if let Some(n) = line
            .strip_prefix("=== ")
            .and_then(|s| s.strip_suffix(" ==="))
        {
            if !name.is_empty() {
                out.push(Block {
                    name: name.clone(),
                    asm: asm.clone(),
                });
            }
            name = n.to_string();
            asm.clear();
            in_bytes = false;
        } else if line.trim() == "--- bytes ---" {
            in_bytes = true;
        } else if !in_bytes {
            asm.push_str(line);
            asm.push('\n');
        }
    }
    if !name.is_empty() {
        out.push(Block { name, asm });
    }
    out
}

/// Assemble one snippet's full source and return its machine-code bytes.
fn assemble(aeon: &Path, asl: &Path, p2bin: &Path, work: &Path, src: &str) -> Vec<u8> {
    let asm = work.join("gen.asm");
    let p = work.join("gen.p");
    let lst = work.join("gen.lst");
    let bin = work.join("gen.bin");
    let _ = fs::remove_file(&p);
    let _ = fs::remove_file(&bin);

    fs::write(&asm, src).expect("write snippet");

    let asl_out = Command::new(asl)
        .current_dir(aeon)
        .env("AS_MSGPATH", "tools")
        .env("USEANSI", "n")
        .args([
            "-cpu",
            "68000",
            "-q",
            "-L",
            "-U",
            "-olist",
            lst.to_str().unwrap(),
            "-o",
            p.to_str().unwrap(),
            asm.to_str().unwrap(),
        ])
        .output()
        .expect("run asl");
    assert!(
        asl_out.status.success(),
        "asl failed for {src:?}:\n{}",
        String::from_utf8_lossy(&asl_out.stderr)
    );

    let p2b_out = Command::new(p2bin)
        .arg(&p)
        .arg(&bin)
        .output()
        .expect("run p2bin");
    assert!(
        p2b_out.status.success(),
        "p2bin failed for {src:?}:\n{}",
        String::from_utf8_lossy(&p2b_out.stderr)
    );

    fs::read(&bin).expect("read bin")
}
