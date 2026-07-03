//! `gen-m68k-vectors` — regenerate the committed 68000 golden-vector oracle from asl.
//!
//! MANUAL developer tool — NOT run in CI. It derives snippet strings from the shared
//! canonical `corpus_m68k()`; for each it assembles a `cpu 68000 / org 0` snippet with
//! the real `asl` (asl 1.42) from the Aeon tree, extracts bytes with `p2bin`, and
//! (over)writes `tests/m68k_golden_vectors.txt` as `<snippet> => <uppercase hex>` in
//! `corpus_m68k()` order. Commit the result. CI reads the committed file (never asl).
//!
//! ```text
//! cargo run -p sigil-isa --bin gen-m68k-vectors
//! AEON_DIR=/path/to/aeon cargo run -p sigil-isa --bin gen-m68k-vectors
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "../../tests/corpus_m68k/mod.rs"]
mod corpus_m68k;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let golden_path = manifest.join("tests/m68k_golden_vectors.txt");

    let aeon = std::env::var("AEON_DIR")
        .unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    let aeon = PathBuf::from(aeon);
    let asl = aeon.join("tools/asl");
    let p2bin = aeon.join("tools/p2bin");
    assert!(asl.is_file(), "asl not found at {} (set AEON_DIR)", asl.display());
    assert!(p2bin.is_file(), "p2bin not found at {} (set AEON_DIR)", p2bin.display());

    let work = std::env::temp_dir().join("sigil_m68k_gen");
    fs::create_dir_all(&work).expect("create work dir");

    let mut out = String::new();
    let mut count = 0usize;
    for (snippet, _inst) in corpus_m68k::corpus_m68k() {
        let bytes = assemble(&aeon, &asl, &p2bin, &work, snippet);
        let hex = bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");
        out.push_str(snippet);
        out.push_str(" => ");
        out.push_str(&hex);
        out.push('\n');
        count += 1;
    }

    fs::write(&golden_path, &out).expect("write golden file");
    eprintln!("wrote {count} vectors to {}", golden_path.display());
}

/// Assemble a single 68000 snippet at `org 0` and return its machine-code bytes.
fn assemble(aeon: &Path, asl: &Path, p2bin: &Path, work: &Path, snippet: &str) -> Vec<u8> {
    let asm = work.join("gen.asm");
    let p = work.join("gen.p");
    let lst = work.join("gen.lst");
    let bin = work.join("gen.bin");
    let _ = fs::remove_file(&p);
    let _ = fs::remove_file(&bin);

    let src = format!("        cpu 68000\n        org 0\n        {snippet}\n");
    fs::write(&asm, src).expect("write snippet");

    let asl_out = Command::new(asl)
        .current_dir(aeon)
        .env("AS_MSGPATH", "tools")
        .env("USEANSI", "n")
        .args([
            "-cpu", "68000", "-q", "-L", "-U",
            "-olist", lst.to_str().unwrap(),
            "-o", p.to_str().unwrap(),
            asm.to_str().unwrap(),
        ])
        .output()
        .expect("run asl");
    assert!(
        asl_out.status.success(),
        "asl failed for {snippet:?}:\n{}",
        String::from_utf8_lossy(&asl_out.stderr)
    );

    let p2b_out = Command::new(p2bin).arg(&p).arg(&bin).output().expect("run p2bin");
    assert!(
        p2b_out.status.success(),
        "p2bin failed for {snippet:?}:\n{}",
        String::from_utf8_lossy(&p2b_out.stderr)
    );

    fs::read(&bin).expect("read bin")
}
