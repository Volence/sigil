//! A build reads its modules from its two named roots and nothing else (G0b Task 9, spec
//! §5.3; closes G0a A-I2 `G0A-SCAN-READS-OTHER-GAME`). Before, the game-as-data driver
//! walked the whole engine checkout, so building `games/demo` parsed every sonic4 module
//! (and aeon's `tools/fixtures/`), and scoped candidates by id prefix afterwards.
//!
//! Reference-dependent (`AEON_DIR`, `sigil_build::reference::aeon_dir`): a reference tree
//! without the two games skips with the SKIP marker, a failure under `SIGIL_STRICT_GATE=1`.
//!
//! Runner: `cargo test -p sigil-build` (the landing bar's full suite).

use sigil_build::driver::load_project;
use sigil_span::roots::RootName;
use std::path::PathBuf;

fn reference_tree(rels: &[&str]) -> Option<PathBuf> {
    let aeon = sigil_build::reference::aeon_dir();
    if let Some(missing) = rels.iter().find(|rel| !aeon.join(rel).exists()) {
        let path = aeon.join(missing);
        assert!(std::env::var_os("SIGIL_STRICT_GATE").is_none(), "SIGIL_STRICT_GATE set but reference missing: {}", path.display());
        eprintln!("skip: reference not at {} (set AEON_DIR)", path.display());
        return None;
    }
    Some(aeon)
}

/// `games/demo`'s scan holds the engine's modules and demo's, each tagged with its root,
/// and no module of sonic4 (by id or by path) or of any other directory of the checkout.
/// The control: the same checkout walked whole DOES hold sonic4's modules, so the absence
/// is measured, not vacuous.
#[test]
fn engine_root_alone_does_not_pull_other_games() {
    let Some(aeon) = reference_tree(&["games/demo/game.toml", "games/sonic4/game.toml"]) else { return };
    let env = |_: &str| None;
    let project = load_project(&aeon.join("games/demo"), &env).expect("load games/demo");
    let (manifest, diags) = project.scan();
    let errors: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errors.is_empty(), "the demo scan failed: {errors:?}");

    let (whole, _) = sigil_frontend_emp::resolve::manifest::Manifest::scan(&aeon);
    let whole_s4 = whole.modules.iter().filter(|m| m.id.starts_with("games.sonic4.")).count();
    assert!(whole_s4 >= 50, "UNMEASURABLE: the whole checkout holds only {whole_s4} sonic4 modules");

    let engine = std::fs::canonicalize(&project.engine_dir).unwrap();
    let game = std::fs::canonicalize(project.sources_root()).unwrap();
    let sonic4 = std::fs::canonicalize(aeon.join("games/sonic4")).unwrap();
    let mut faults = Vec::new();
    let (mut n_engine, mut n_game) = (0, 0);
    for m in &manifest.modules {
        let path = std::fs::canonicalize(&m.path).unwrap();
        if m.id.starts_with("games.sonic4") || path.starts_with(&sonic4) {
            faults.push(format!("sonic4's `{}` ({}) is in demo's scan", m.id, m.path.display()));
        }
        match m.root {
            Some(RootName::Engine) if path.starts_with(&engine) => n_engine += 1,
            Some(RootName::Game) if path.starts_with(&game) => n_game += 1,
            other => faults.push(format!("`{}` ({}) is tagged {other:?}, outside its root", m.id, m.path.display())),
        }
    }
    let s4 = faults.iter().filter(|f| f.starts_with("sonic4's")).count();
    assert!(faults.is_empty(), "{} fault(s), {s4} of them sonic4 modules:\n  {}", faults.len(), faults.join("\n  "));
    assert!(n_engine >= 30 && n_game >= 3, "UNMEASURABLE: {n_engine} engine and {n_game} demo modules");
}
