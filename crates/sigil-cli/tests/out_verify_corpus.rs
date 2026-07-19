//! Contract-grammar v2 §G4.5 — the callee-side `out()` production residue over
//! the REAL aeon corpus. Checkpoint-B inspection: DUMP every `[proc.out-
//! unverified]` firing for adjudication (not yet an assert-empty gate).

use sigil_frontend_emp::corpus_contracts::analyze_corpus;
use sigil_frontend_emp::parse_str;
use std::path::{Path, PathBuf};

fn emp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == ".worktrees") {
                continue;
            }
            emp_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

#[test]
fn dump_out_unverified_residue() {
    let Ok(aeon) = std::env::var("AEON_DIR") else {
        eprintln!("skip: AEON_DIR not set");
        return;
    };
    let aeon = PathBuf::from(aeon);
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no .emp files under {}", aeon.display());

    let files: Vec<_> =
        paths.iter().map(|p| parse_str(&std::fs::read_to_string(p).unwrap()).0).collect();
    let r = analyze_corpus(&files);

    eprintln!("=== [proc.out-unverified] residue: {} firing(s) ===", r.out_firings.len());
    for f in &r.out_firings {
        eprintln!("  {} :: out({}) — {}", f.proc, f.reg, f.reason);
    }
    eprintln!("=== [call.live-clobbered] D1c: {} firing(s) ===", r.live_clobbered_firings.len());
    for f in &r.live_clobbered_firings {
        eprintln!("  {} @ {} :: {}", f.proc, f.callee, f.reg);
    }
}
