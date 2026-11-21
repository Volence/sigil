//! The no-game-names gate's id fixture (`tests/support/game_ids.txt`) is generated from
//! the aeon descriptions, and this test holds it there (G0b Task 12).
//!
//! `tests/no_game_names.rs` runs in CI and reads only sigil's source and the fixture. The
//! fixture's `aeon:` rows are re-derived here from a reference tree (the A8 `AEON_DIR`
//! guard): one `game` row per `games/<dir>/game.toml`, its `id` read by the driver's own
//! description parser, and one `engine` row per distinct `engine.suite_path`. Rows not
//! sourced from aeon (`cherub`, built in its own repo) are left alone, but there must be
//! one. On disagreement the test prints the rows to paste.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture_text() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/game_ids.txt");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

#[test]
fn the_fixture_is_the_descriptions_ids() {
    let aeon = sigil_harness::test_support::aeon_dir();
    if !aeon.exists() {
        if sigil_harness::test_support::strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }

    // The rows the descriptions give.
    let games_dir = aeon.join("games");
    let mut want: BTreeSet<String> = BTreeSet::new();
    let mut engines: BTreeSet<String> = BTreeSet::new();
    let mut entries: Vec<_> = std::fs::read_dir(&games_dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", games_dir.display()))
        .map(|e| e.expect("dir entry").path())
        .collect();
    entries.sort();
    for dir in entries {
        let desc = dir.join("game.toml");
        if !desc.is_file() {
            continue;
        }
        let g = sigil_build::describe::load_game(&desc).unwrap_or_else(|e| panic!("{}: {e:?}", desc.display()));
        let rel = desc.strip_prefix(&aeon).expect("under the tree").display().to_string();
        want.insert(format!("game {} aeon:{rel}", g.id));
        engines.insert(g.engine.suite_path.clone());
    }
    assert!(!want.is_empty(), "{} holds no game description: nothing to derive", games_dir.display());
    for e in &engines {
        want.insert(format!("engine {e} aeon:games/*/game.toml#engine.suite_path"));
    }

    // The rows the fixture holds.
    let text = fixture_text();
    let rows: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    let have: BTreeSet<String> = rows.iter().filter(|r| r.split(' ').nth(2).is_some_and(|s| s.starts_with("aeon:"))).cloned().collect();
    let foreign: Vec<&String> = rows.iter().filter(|r| !have.contains(*r)).collect();

    assert!(
        have == want,
        "tests/support/game_ids.txt disagrees with the descriptions in {}.\nThe aeon rows must be exactly:\n{}\nfixture has:\n{}",
        aeon.display(),
        want.iter().cloned().collect::<Vec<_>>().join("\n"),
        have.iter().cloned().collect::<Vec<_>>().join("\n")
    );
    assert!(
        foreign.iter().any(|r| r.starts_with("game cherub ")),
        "the fixture lost its `cherub` row (Cherub is built in its own repo, so aeon cannot supply it)"
    );
    println!("fixture agrees with {}: {} aeon rows, {} other rows", aeon.display(), have.len(), foreign.len());
}
