//! Contract-grammar v2 D1d — the `[proc.dead-save]` lint run over the REAL aeon
//! corpus: the pass-3 dead-save worklist. Prints proc / register / bracketed
//! callees for every firing. The checkpoint measurement (does the lint find the
//! review's named customers — dplc, load_object, children — and what beyond).
//!
//! THIS WALK RUNS PER SHIPPED SHAPE, from sigil-cli. The gate lives in sigil-cli
//! because that crate depends on `sigil-harness` — the owner of the shape `-D`
//! profiles — as well as `sigil-frontend-emp`. It runs the dead-save walk under each
//! shipped shape's own `-D` set (`native::shape_defines`) with that shape's bound L1
//! interface env, so a dead save inside a `DEBUG`/`CRASH_REPORT`/`SOUND_*`-gated arm
//! is dumped when the ROM that ships it turns the arm on — a no-`-D` walk lowers no
//! gated arm and never sees it. The define-free baseline count is reported beside the
//! per-shape counts, so a shape that widens the worklist is visible.
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

/// The reference tree, or `None` when it is absent and strict mode is off (under
/// `SIGIL_STRICT_GATE` a missing tree HARD-FAILS — the house reference-gate pattern).
fn aeon_dir() -> Option<PathBuf> {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    Some(aeon)
}

#[test]
fn dead_save_worklist_over_corpus() {
    let Some(aeon) = aeon_dir() else { return };
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

    // NON-VACUOUS: the walk must reach the dead-save analysis AND find the census the
    // ledger states. The live worklist is 3 firings (TestChurnObj_Main/A0,
    // TileCache_FillColumn/D7, TileCache_WarmupBelowRow/D7) in every shape; a tolerant
    // floor of 3 (not an exact pin — this is a dump, adoption may retire a customer)
    // gives the stated census teeth: a walk that stops running, or one that silently
    // loses a customer, drops below it and fails.
    assert!(
        widest >= 3,
        "expected the 3-firing dead-save census in the widest shape, found only {widest} — \
         the worklist analysis is not running or has silently lost a customer"
    );
}
