//! `sigil-build` NEVER DEPENDS ON `sigil-harness`, directly or through anything it links.
//!
//! G0b moves the generic build driver out of the test crate (Task 4) so the new driver
//! (Task 5) can stand on it without the harness. The move only means something while
//! the edge runs one way: sigil-harness calls sigil-build, and nothing sigil-build
//! builds with reaches back. One `use` of a harness item and one manifest line would
//! put the game profiles, the frozen tables and the reference-tree plumbing back under
//! the driver, and every crate that links sigil-build would inherit them.
//!
//! WHAT IS CHECKED: the dependency graph `cargo tree -p sigil-build` walks, read from the
//! manifests themselves. From `crates/sigil-build/Cargo.toml` every `[dependencies]`,
//! `[dev-dependencies]` and `[build-dependencies]` entry (and their `[target.*]` forms),
//! then, transitively, every path dependency's `[dependencies]` and
//! `[build-dependencies]` (a dependency's dev-dependencies are not built for its
//! dependents). No crate reached may be the package `sigil-harness`, whatever name the
//! entry gives it (`package = ...` renames are followed).
//!
//! LOUD ON UNMEASURABLE: a manifest that cannot be read or parsed, a dependency entry
//! this walk does not know how to follow (`workspace = true`, `git = ...`), and a walk
//! that reached none of the crates sigil-build names are refusals, never a pass. A
//! registry (crates.io) dependency is not followed: no sigil crate is published there.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const FORBIDDEN: &str = "sigil-harness";

/// Every package the walk from `root` (a crate directory) reaches through path
/// dependencies, by package name, with the chain that reached it.
fn reached(root: &Path) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut stack: Vec<(PathBuf, Vec<String>, bool)> = vec![(root.to_path_buf(), Vec::new(), true)];
    while let Some((dir, chain, is_root)) = stack.pop() {
        let dir = std::fs::canonicalize(&dir)
            .unwrap_or_else(|e| panic!("UNMEASURABLE: dependency directory {} does not resolve: {e}", dir.display()));
        if !seen.insert(dir.clone()) {
            continue;
        }
        let manifest = dir.join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest)
            .unwrap_or_else(|e| panic!("UNMEASURABLE: cannot read {}: {e}", manifest.display()));
        let doc: toml::Table = text
            .parse()
            .unwrap_or_else(|e| panic!("UNMEASURABLE: cannot parse {}: {e}", manifest.display()));
        let name = doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or_else(|| panic!("UNMEASURABLE: {} names no [package] name", manifest.display()))
            .to_string();
        let mut chain = chain;
        chain.push(name.clone());
        out.push((name, chain.clone()));

        let kinds: &[&str] =
            if is_root { &["dependencies", "dev-dependencies", "build-dependencies"] } else { &["dependencies", "build-dependencies"] };
        let mut tables: Vec<&toml::Table> = kinds.iter().filter_map(|k| doc.get(*k).and_then(|t| t.as_table())).collect();
        if let Some(targets) = doc.get("target").and_then(|t| t.as_table()) {
            for cfg in targets.values().filter_map(|v| v.as_table()) {
                tables.extend(kinds.iter().filter_map(|k| cfg.get(*k).and_then(|t| t.as_table())));
            }
        }
        for table in tables {
            for (key, spec) in table {
                // A bare version string, or a table with only `version`: a registry crate.
                let Some(spec) = spec.as_table() else { continue };
                let package = spec.get("package").and_then(|p| p.as_str()).unwrap_or(key);
                if let Some(path) = spec.get("path").and_then(|p| p.as_str()) {
                    stack.push((dir.join(path), chain.clone(), false));
                } else if spec.contains_key("workspace") || spec.contains_key("git") {
                    panic!(
                        "UNMEASURABLE: {} declares `{key}` (package `{package}`) through a form this walk \
                         does not follow ({spec}); teach the walk that form rather than skipping the entry",
                        manifest.display()
                    );
                }
            }
        }
    }
    out
}

/// The crates the walk reached that are the forbidden package, each with its chain.
fn forbidden_edges(root: &Path) -> Vec<String> {
    reached(root)
        .into_iter()
        .filter(|(name, _)| name == FORBIDDEN)
        .map(|(_, chain)| chain.join(" -> "))
        .collect()
}

#[test]
fn sigil_build_reaches_no_sigil_harness() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reached = reached(root);
    let names: BTreeSet<&str> = reached.iter().map(|(n, _)| n.as_str()).collect();
    // The walk must actually have walked: the crates sigil-build's manifest names
    // directly are what it is built from, so missing any of them means the walk read
    // nothing, not that nothing is wrong.
    for direct in ["sigil-span", "sigil-ir", "sigil-link", "sigil-frontend-emp"] {
        assert!(
            names.contains(direct),
            "UNMEASURABLE: the walk from {} never reached `{direct}`, which sigil-build depends on; \
             it reached only {names:?}",
            root.display()
        );
    }
    // ...and must have RECURSED: a walk that stopped at the root's own entries would
    // pass every check above while never looking at what those crates depend on. The
    // root's direct entries come from its manifest, so the comparison is not a typed list.
    let doc: toml::Table = std::fs::read_to_string(root.join("Cargo.toml"))
        .expect("UNMEASURABLE: cannot re-read sigil-build's manifest")
        .parse()
        .expect("UNMEASURABLE: cannot re-parse sigil-build's manifest");
    let direct = ["dependencies", "dev-dependencies", "build-dependencies"]
        .iter()
        .filter_map(|k| doc.get(*k).and_then(|t| t.as_table()))
        .flat_map(|t| t.values())
        .filter(|v| v.get("path").is_some())
        .count();
    assert!(
        names.len() > direct + 1,
        "UNMEASURABLE: the walk reached {} crate(s), no more than sigil-build and its {direct} direct path \
         dependencies, so it did not recurse into them: {names:?}",
        names.len()
    );
    let bad = forbidden_edges(root);
    assert!(
        bad.is_empty(),
        "sigil-build depends on {FORBIDDEN}: {bad:?}. The driver left the test crate (G0b Task 4) and \
         must not reach back into it; move what it needs into sigil-build instead"
    );
    eprintln!("no_harness_edge: {} crate(s) reached from sigil-build, none is {FORBIDDEN}", names.len());
}

/// A scratch directory under this binary's target directory (on disk, never the
/// system temp directory, which is tmpfs here).
fn scratch(tag: &str) -> PathBuf {
    let exe = std::env::current_exe().expect("UNMEASURABLE: this test has no path");
    let target = exe.ancestors().nth(3).expect("UNMEASURABLE: the test binary is not under a target directory");
    let dir = target.join("no-harness-edge").join(format!("{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir scratch");
    dir
}

fn write_crate(dir: &Path, name: &str, deps: &str) {
    std::fs::create_dir_all(dir).expect("mkdir crate");
    std::fs::write(dir.join("Cargo.toml"), format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n{deps}"))
        .expect("write manifest");
}

/// The walk sees a TRANSITIVE edge and a RENAMED one, and does not see a dependency's
/// dev-dependency (cargo does not build it for the dependent). A control fixture with
/// none of those edges walks clean, so the red cases are red because of the edge.
#[test]
fn the_walk_finds_a_transitive_or_renamed_edge_and_ignores_a_dependencys_dev_dependency() {
    let dir = scratch("fixture");
    write_crate(&dir.join("harness"), FORBIDDEN, "");

    // Control: root -> mid, and mid only DEV-depends on the harness.
    write_crate(&dir.join("root"), "root", "[dependencies]\nmid = { path = \"../mid\" }\n");
    write_crate(&dir.join("mid"), "mid", "[dev-dependencies]\nh = { path = \"../harness\" }\n");
    assert_eq!(forbidden_edges(&dir.join("root")), Vec::<String>::new(), "control fixture must walk clean");

    // Transitive: mid now really depends on it.
    write_crate(&dir.join("mid"), "mid", "[dependencies]\nh = { path = \"../harness\" }\n");
    assert_eq!(forbidden_edges(&dir.join("root")), vec![format!("root -> mid -> {FORBIDDEN}")]);

    // Renamed, at the root, as a dev-dependency (the root's dev-dependencies count).
    write_crate(&dir.join("mid"), "mid", "");
    write_crate(&dir.join("root"), "root", "[dev-dependencies]\nnot_the_harness = { path = \"../harness\" }\n");
    assert_eq!(forbidden_edges(&dir.join("root")), vec![format!("root -> {FORBIDDEN}")]);

    let _ = std::fs::remove_dir_all(&dir);
}
