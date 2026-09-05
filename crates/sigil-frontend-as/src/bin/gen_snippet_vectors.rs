//! `gen-snippet-vectors` — regenerate the committed snippet golden bytes from asl.
//!
//! MANUAL developer tool — NOT run in CI. It reads the `asm` blocks from
//! `tests/snippets_golden.txt`, assembles each one with the real `asl` (asl 1.42),
//! extracts the emitted bytes with `p2bin`, and rewrites each block's
//! `--- bytes ---` section in place. The CI test (`tests/asl_snippets.rs`) reads
//! the committed golden bytes instead and never needs asl.
//!
//! **OUT-OF-REPO asl (Stage-3 P4d / OQ-A).** The `asl`/`p2bin` binaries were
//! DELETED from the aeon tree at the flip (nothing-retained). The committed golden
//! vectors are the frozen independent-asl witness; EXTENDING the corpus (a new
//! snippet block for a post-flip instruction shape) requires asl out-of-repo:
//! install the public Macro Assembler AS
//! (<http://john.ccac.rwth-aachen.de:8000/as/>) and point `ASL_BIN` (and, if not a
//! sibling, `P2BIN_BIN`) at the binaries. Fail-loud otherwise — never a silent skip.
//!
//! ```text
//! ASL_BIN=/opt/asl/bin/asl cargo run -p sigil-frontend-as --bin gen-snippet-vectors
//! ```
//!
//! The committed golden bytes are **generator-produced from real asl** and
//! regenerate byte-identically (running this tool on the committed file is a
//! git-clean no-op — the non-circularity invariant: each new snippet block must
//! churn ONLY its own bytes, proving every committed golden is authentic asl
//! output, not a value the implementation happened to emit). That no-op holds
//! **for the build named in the file's provenance header**; run under a
//! different asl the header churns too, which is the header earning its place —
//! it makes a silent dependency visible rather than adding one.
//!
//! **It refuses to write a golden it cannot stamp.** `ASL_BIN` still names any
//! build; an unidentified instrument is what made the old goldens unable to say
//! which independent implementation witnessed them. See `asl_provenance`.

// `sigil-frontend-as` does not depend on `sigil-isa` and neither crate carries
// third-party dependencies, so the provenance helper is shared by path — the
// same device the isa generators use for their corpus modules. The module is
// self-contained std-only code, so both inclusion modes compile it identically.
#[path = "../../../sigil-isa/src/asl_provenance.rs"]
mod asl_provenance;

use asl_provenance::Provenance;
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

    // asl/p2bin are OUT-OF-REPO since the flip (P4d/OQ-A) — take them from ASL_BIN /
    // P2BIN_BIN (p2bin defaults to a sibling of asl). Fail loud, never silently skip.
    let asl = match std::env::var("ASL_BIN") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            eprintln!(
                "gen-snippet-vectors: set ASL_BIN to the Macro Assembler AS binary to mint new \
                 vectors.\n  asl was DELETED from the repo at the flip (nothing-retained). Install \
                 AS (http://john.ccac.rwth-aachen.de:8000/as/) and point ASL_BIN at it; the \
                 committed vectors remain the frozen witness. (Stage-3 P4d / OQ-A.)"
            );
            std::process::exit(2);
        }
    };
    let p2bin = std::env::var("P2BIN_BIN").map(PathBuf::from).unwrap_or_else(|_| {
        asl.parent().map(|d| d.join("p2bin")).unwrap_or_else(|| PathBuf::from("p2bin"))
    });
    assert!(asl.is_file(), "ASL_BIN not a file: {} (install AS, set ASL_BIN)", asl.display());
    assert!(
        p2bin.is_file(),
        "p2bin not found at {} (set P2BIN_BIN, or place it beside ASL_BIN)",
        p2bin.display()
    );

    // Identify the instrument BEFORE minting anything: a golden that cannot be
    // stamped must not be written at all, and the refusal must land before the
    // committed file is touched.
    let prov = match Provenance::capture("gen_snippet_vectors", &asl, &p2bin) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "gen-snippet-vectors: cannot identify the toolchain, so nothing was written.\n  \
                 {e}\n  A golden whose instrument is unrecorded is the defect this refuses."
            );
            std::process::exit(4);
        }
    };
    // The paths go to stderr, not into the file: they are context for whoever is
    // running the mint, and are not a property of the binary (see asl_provenance).
    eprintln!("gen-snippet-vectors: asl   {} md5 {}", asl.display(), prov.asl.md5);
    eprintln!("gen-snippet-vectors: p2bin {} md5 {}", p2bin.display(), prov.p2bin.md5);

    let work = std::env::temp_dir().join("sigil_snippet_gen");
    fs::create_dir_all(&work).expect("create work dir");

    let mut out = prov.header();
    for b in &blocks {
        let bytes = assemble(&asl, &p2bin, &work, &b.asm);
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
///
/// Everything before the first `=== ` header is the provenance header and is
/// dropped: this run writes a fresh one describing the asl it was handed, which
/// is the whole point — the header must describe THIS mint, never be carried
/// over from the previous one.
fn parse_blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut name = String::new();
    let mut asm = String::new();
    let mut in_bytes = false;
    let mut seen_first_block = false;
    for line in text.lines() {
        if !seen_first_block && !line.starts_with("=== ") {
            continue;
        }
        if let Some(n) = line
            .strip_prefix("=== ")
            .and_then(|s| s.strip_suffix(" ==="))
        {
            seen_first_block = true;
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
fn assemble(asl: &Path, p2bin: &Path, work: &Path, src: &str) -> Vec<u8> {
    let asm = work.join("gen.asm");
    let p = work.join("gen.p");
    let lst = work.join("gen.lst");
    let bin = work.join("gen.bin");
    let _ = fs::remove_file(&p);
    let _ = fs::remove_file(&bin);

    fs::write(&asm, src).expect("write snippet");

    // Self-contained snippets — assemble from the work dir. AS finds its message
    // catalogs via AS_MSGPATH; point it at ASL_BIN's directory (a normal AS install
    // keeps `as.msg` beside the binary), overridable for a non-standard layout.
    let msgpath = std::env::var("AS_MSGPATH")
        .unwrap_or_else(|_| asl.parent().map(|d| d.display().to_string()).unwrap_or_default());
    let asl_out = Command::new(asl)
        .current_dir(work)
        .env("AS_MSGPATH", msgpath)
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
