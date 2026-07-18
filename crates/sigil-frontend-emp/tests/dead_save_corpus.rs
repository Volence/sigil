//! Contract-grammar v2 D1d — the `[proc.dead-save]` lint run over the REAL aeon
//! corpus: the pass-3 dead-save worklist. Prints proc / register / bracketed
//! callees for every firing. The checkpoint measurement (does the lint find the
//! review's named customers — dplc, load_object, children — and what beyond).
//!
//! Gated on `AEON_DIR`.

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
fn dead_save_worklist_over_corpus() {
    let Ok(aeon) = std::env::var("AEON_DIR") else {
        eprintln!("skip: AEON_DIR not set");
        return;
    };
    let aeon = PathBuf::from(aeon);
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    let files: Vec<_> = paths
        .iter()
        .map(|p| parse_str(&std::fs::read_to_string(p).unwrap()).0)
        .collect();
    let r = analyze_corpus(&files);

    let mut report = format!("\n== [proc.dead-save] worklist ({} firings) ==\n", r.dead_saves.len());
    for d in &r.dead_saves {
        report.push_str(&format!(
            "  {:<26} save {:?}  around {}\n",
            d.proc,
            d.reg,
            d.callees.join(", ")
        ));
    }
    eprintln!("{report}");
}
