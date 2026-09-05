//! `gen-z80-vectors` — regenerate the committed Z80 golden-vector oracle from asl.
//!
//! MANUAL developer tool — NOT run in CI. It derives its snippet strings from the
//! shared canonical `corpus()` (the SAME list the CI test and Task 9's completeness
//! gate consume — no second, drifting snippet list); for each snippet it assembles a
//! `cpu z80 / phase 0` snippet with the real `asl` (asl 1.42, out-of-repo — see ASL_BIN, P4d/OQ-A),
//! extracts the exact bytes with `p2bin`, and (over)writes
//! `tests/z80_golden_vectors.txt` as `<snippet> => <space-separated uppercase hex>`,
//! in `corpus()` order, under a provenance header naming the asl build that
//! answered. Commit the result.
//!
//! **It refuses to write a golden it cannot stamp.** `ASL_BIN` still names any
//! build — that hook is the point, since asl is out-of-repo since the P4d flip —
//! but an unidentified instrument is what made the old goldens unable to say
//! which independent implementation witnessed them. See `sigil_isa::asl_provenance`.
//!
//! ```text
//! ASL_BIN=/opt/asl/bin/asl cargo run -p sigil-isa --bin gen-z80-vectors
//! ```
//!
//! CI reads the committed file instead (see `tests/z80_golden.rs`); it never
//! needs asl.

use sigil_isa::asl_provenance::Provenance;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// The single source of truth for the snippet list. Cargo does not compile
// `tests/corpus/mod.rs` as its own target, so the generator includes it directly;
// `sigil_isa` (the lib) is available to this bin, so the module's `use
// sigil_isa::z80::*` resolves. The generator uses only the snippet strings.
#[path = "../../tests/corpus/mod.rs"]
mod corpus;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let golden_path = manifest.join("tests/z80_golden_vectors.txt");

    // asl/p2bin are OUT-OF-REPO since the flip (P4d/OQ-A) — take them from ASL_BIN /
    // P2BIN_BIN (p2bin defaults to a sibling of asl). Fail loud, never silently skip.
    let Ok(asl) = std::env::var("ASL_BIN").map(PathBuf::from) else {
        eprintln!(
            "gen-z80-vectors: set ASL_BIN to the Macro Assembler AS binary to mint new vectors.\n  \
             asl was DELETED from the repo at the flip (nothing-retained). Install AS \
             (http://john.ccac.rwth-aachen.de:8000/as/) and point ASL_BIN at it; the committed \
             vectors remain the frozen witness. (Stage-3 P4d / OQ-A.)"
        );
        std::process::exit(2);
    };
    let p2bin = std::env::var("P2BIN_BIN").map(PathBuf::from).unwrap_or_else(|_| {
        asl.parent().map(|d| d.join("p2bin")).unwrap_or_else(|| PathBuf::from("p2bin"))
    });
    assert!(asl.is_file(), "ASL_BIN not a file: {} (install AS, set ASL_BIN)", asl.display());
    assert!(p2bin.is_file(), "p2bin not found at {} (set P2BIN_BIN)", p2bin.display());

    // Identify the instrument BEFORE minting anything: a golden that cannot be
    // stamped must not be written at all, and the refusal must land before the
    // committed file is touched.
    let prov = match Provenance::capture("gen-z80-vectors", &asl, &p2bin) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "gen-z80-vectors: cannot identify the toolchain, so nothing was written.\n  \
                 {e}\n  A golden whose instrument is unrecorded is the defect this refuses."
            );
            std::process::exit(4);
        }
    };
    // The paths go to stderr, not into the file: they are context for whoever is
    // running the mint, and are not a property of the binary (see asl_provenance).
    eprintln!("gen-z80-vectors: asl   {} md5 {}", asl.display(), prov.asl.md5);
    eprintln!("gen-z80-vectors: p2bin {} md5 {}", p2bin.display(), prov.p2bin.md5);

    let work = std::env::temp_dir().join("sigil_z80_gen");
    fs::create_dir_all(&work).expect("create work dir");

    let mut out = prov.header();
    let mut count = 0usize;
    for (snippet, _inst) in corpus::corpus() {
        let bytes = assemble(&asl, &p2bin, &work, snippet);
        let hex = bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(snippet);
        out.push_str(" => ");
        out.push_str(&hex);
        out.push('\n');
        count += 1;
    }

    fs::write(&golden_path, &out).expect("write golden file");
    eprintln!("wrote {count} vectors to {}", golden_path.display());
}

/// Assemble a single Z80 snippet at `phase 0` and return its machine-code bytes.
fn assemble(asl: &Path, p2bin: &Path, work: &Path, snippet: &str) -> Vec<u8> {
    let asm = work.join("gen.asm");
    let p = work.join("gen.p");
    let lst = work.join("gen.lst");
    let bin = work.join("gen.bin");
    let _ = fs::remove_file(&p);
    let _ = fs::remove_file(&bin);

    let src = format!("        cpu z80\n        phase 0\n        {snippet}\n");
    fs::write(&asm, src).expect("write snippet");

    let msgpath = std::env::var("AS_MSGPATH")
        .unwrap_or_else(|_| asl.parent().map(|d| d.display().to_string()).unwrap_or_default());
    let asl_out = Command::new(asl)
        .current_dir(work)
        .env("AS_MSGPATH", msgpath)
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

    let p2b_out = Command::new(p2bin)
        .arg(&p)
        .arg(&bin)
        .output()
        .expect("run p2bin");
    assert!(
        p2b_out.status.success(),
        "p2bin failed for {snippet:?}:\n{}",
        String::from_utf8_lossy(&p2b_out.stderr)
    );

    fs::read(&bin).expect("read bin")
}
