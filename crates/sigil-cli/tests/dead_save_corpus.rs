//! Contract-grammar v2 D1d — the `[proc.dead-save]` lint run over the REAL aeon
//! corpus: the pass-3 dead-save worklist. Prints proc / register / bracketed
//! callees for every firing. The checkpoint measurement (does the lint find the
//! review's named customers — dplc, load_object, children — and what beyond).
//!
//! WHY PER SHAPE, IN sigil-cli. Its earlier home (`sigil-frontend-emp/tests`)
//! cannot depend on `sigil-harness`, so it could only call the no-`-D`
//! `analyze_corpus` — a dead save inside a `DEBUG`/`CRASH_REPORT`/`SOUND_*`-gated
//! arm never lowered and so was never on the worklist. Here it re-runs the walk
//! under each shipped shape's own `-D` set (`native::shape_defines`) with that
//! shape's bound L1 interface env, so a debug-only dead save is dumped when the ROM
//! that ships it turns the arm on. The define-free baseline count is reported beside
//! the per-shape counts, so a shape that widens the worklist is visible.
//!
//! Reference tree: defaults to the sibling aeon checkout (override with `AEON_DIR`);
//! under `SIGIL_STRICT_GATE` a missing tree HARD-FAILS so the dump runs in the
//! standard strict invocation.

use sigil_frontend_emp::corpus_contracts::{
    analyze_corpus, analyze_corpus_with_contracts, bind_corpus_interfaces,
};
use sigil_frontend_emp::parse_str;
use sigil_harness::native;
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
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    let files: Vec<_> = paths
        .iter()
        .map(|p| parse_str(&std::fs::read_to_string(p).unwrap()).0)
        .collect();

    // The DEFINE-FREE baseline (this gate's earlier no-`-D` worklist).
    let base = analyze_corpus(&files);
    let mut report = format!(
        "\n== [proc.dead-save] worklist ==\ndefine-free baseline: {} firing(s)\n",
        base.dead_saves.len()
    );
    for d in &base.dead_saves {
        report.push_str(&format!("  {:<26} save {:?}  around {}\n", d.proc, d.reg, d.callees.join(", ")));
    }

    // Per shape: re-run the walk under the shape's `-D` set + bound L1 interface
    // env, so a dead save gated behind a comptime arm is scanned when the ROM that
    // ships it turns the arm on. The WIDEST worklist is the true dead-save census.
    let mut widest = base.dead_saves.len();
    for (label, profile) in native::shipped_shapes() {
        let defines = native::shape_defines(&profile);
        let (iface_env, bind_diags) =
            bind_corpus_interfaces(&files, &defines, profile.game_module_prefix());
        assert!(
            bind_diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "shape `{label}`: L1 bind errors: {bind_diags:?}"
        );
        let r = analyze_corpus_with_contracts(&files, &defines, &iface_env);
        report.push_str(&format!("shape {label}: {} firing(s)\n", r.dead_saves.len()));
        for d in &r.dead_saves {
            report.push_str(&format!(
                "  [{label}] {:<26} save {:?}  around {}\n",
                d.proc,
                d.reg,
                d.callees.join(", ")
            ));
        }
        widest = widest.max(r.dead_saves.len());
    }
    eprintln!("{report}");

    // NON-VACUOUS: the walk must actually reach the dead-save analysis. The corpus
    // has a live worklist (the review's named customers), so an empty result under
    // every shape means the analysis stopped running, not that the corpus is clean.
    assert!(
        widest > 0,
        "no [proc.dead-save] firing under any shape — the worklist analysis is not running"
    );
}
