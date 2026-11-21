//! The G0b descriptions in the aeon REFERENCE tree load, and carry real content.
//!
//! Reference-dependent: the tree is `AEON_DIR` (or `EMPYREAN_SUITE_ROOT/aeon`), resolved by
//! `sigil_build::reference::aeon_dir`, a mirror of sigil-harness's A8 / d-18 resolver (this
//! crate may not depend on sigil-harness). A missing description skips with the SKIP marker,
//! and is a failure under `SIGIL_STRICT_GATE=1`.
//!
//! Until G0b Task 11 every expectation here was read out of the profile-driven driver's Rust
//! source as text (its shape counts, define literals, alignment rows, D1c tables, helper
//! lists, sound bank-id consts, module ids). Those tables are deleted with that driver, so
//! the agree-with-Rust halves retired (their last green output is quoted in
//! `docs/superpowers/notes/2026-11-17-g0b-game-as-data.md`); what stays proves the loaders
//! read real content rather than defaults, and the facts that are structural rather than
//! transcribed.

use sigil_build::describe::{load_baseline, load_engine, load_game, resolve_shape};
use std::path::PathBuf;

fn strict_gate() -> bool {
    std::env::var_os("SIGIL_STRICT_GATE").is_some()
}

/// The guard every test here opens with: `Some(aeon)` when the reference tree carries every
/// path in `rels`, `None` (skip) when it does not, a panic naming the path under
/// `SIGIL_STRICT_GATE=1`. Mirrors sigil-harness `test_support::reference_tree`.
fn reference_tree(rels: &[&str]) -> Option<PathBuf> {
    let aeon = sigil_build::reference::aeon_dir();
    if let Some(missing) = rels.iter().find(|rel| !aeon.join(rel).exists()) {
        let path = aeon.join(missing);
        assert!(!strict_gate(), "SIGIL_STRICT_GATE set but reference missing: {}", path.display());
        eprintln!("skip: reference not at {} (set AEON_DIR)", path.display());
        return None;
    }
    Some(aeon)
}

#[test]
fn sonic4_game_toml_loads() {
    let Some(tree) = reference_tree(&["games/sonic4/game.toml", "games/demo/game.toml", "engine/engine.toml"]) else {
        return;
    };
    let s4 = load_game(&tree.join("games/sonic4/game.toml")).unwrap_or_else(|e| panic!("{e}"));
    let demo = load_game(&tree.join("games/demo/game.toml")).unwrap_or_else(|e| panic!("{e}"));
    let engine = load_engine(&tree.join("engine/engine.toml")).unwrap_or_else(|e| panic!("{e}"));

    // Real content: both games describe shapes and a ring-buffer define, and differ there.
    assert!(!s4.shapes.is_empty() && !demo.shapes.is_empty(), "both games describe shapes");
    assert_ne!(s4.defines["MAX_RING_BUFFER"], demo.defines["MAX_RING_BUFFER"], "the two games differ here");

    // Alignment exceptions are powers of two above 1 (the loader refuses anything else),
    // the engine's and sonic4's rows together are non-empty, and demo carries none of the
    // sound banks.
    assert!(engine.align.iter().chain(&s4.align).count() > 0, "no alignment exception is described");
    assert!(demo.align.is_empty(), "demo carries none of the sound banks");

    // Every shape flattens, and the fixtures are exactly the three ruled ones.
    for g in [&s4, &demo] {
        for name in g.shapes.keys() {
            resolve_shape(g, name).unwrap_or_else(|e| panic!("{}: {e}", g.id));
        }
    }
    let fixtures: Vec<&str> =
        s4.shapes.keys().filter(|n| resolve_shape(&s4, n).unwrap().fixture).map(String::as_str).collect();
    assert_eq!(fixtures, ["config-a-keystones-flipped", "stress-art", "stress-evict"]);

    // The baselines each description names load, with rows in them.
    let eb = load_baseline(&tree.join("engine").join(&engine.baseline)).unwrap_or_else(|e| panic!("{e}"));
    let sb = load_baseline(&tree.join("games/sonic4").join(&s4.baseline)).unwrap_or_else(|e| panic!("{e}"));
    let db = load_baseline(&tree.join("games/demo").join(&demo.baseline)).unwrap_or_else(|e| panic!("{e}"));
    assert!(!eb.d1c.is_empty() || !sb.d1c.is_empty(), "the sonic4 D1c baseline is empty");
    assert!(!eb.d1c_debug.is_empty(), "the engine's debug-only D1c rows are empty");
    assert!(!db.d1c.is_empty(), "demo's D1c rows are empty");
    assert!(!eb.z80_out_unverified.is_empty(), "the engine's Z80 out-unverified rows are empty");
}

#[test]
fn engine_toml_loads() {
    let Some(tree) = reference_tree(&["engine/engine.toml"]) else {
        return;
    };
    let e = load_engine(&tree.join("engine/engine.toml")).unwrap_or_else(|e| panic!("{e}"));
    assert!(!e.helpers.is_empty() && !e.ram_helpers.is_empty(), "the helper lists are empty");
    assert!(!e.struct_twins.is_empty(), "no struct twin is described");
    assert!(!e.sound.blob_bank_ids.is_empty(), "no blob bank-id const is described");
    for id in [&e.terminus, &e.error_handler, &e.release_fault] {
        assert!(id.starts_with("engine."), "`{id}` is not an engine module id");
    }
    // The blob's label and length are engine facts with no Rust spelling (G0b Task 11
    // deleted the constants this used to compare against): the description must state both.
    assert!(!e.error_handler_blob.is_empty() && e.error_handler_blob_len > 0, "the error-handler blob is not described");
    let engine_root = std::path::Path::new("engine");
    let convsym: std::path::PathBuf = engine_root.join(&e.convsym).components().fold(std::path::PathBuf::new(), |mut p, c| {
        match c {
            std::path::Component::ParentDir => {
                p.pop();
            }
            c => p.push(c),
        }
        p
    });
    assert_eq!(convsym, std::path::Path::new("tools/convsym"), "engine.toml `convsym` must name the checkout's tools/convsym (appendix.rs)");
    assert!(!e.gate.is_empty() && e.required_labels.contains(&"EndOfRom".to_string()));
}
