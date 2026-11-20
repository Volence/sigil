//! The per-game `[defines]` mechanism, end to end through the game-as-data driver's
//! define merge (`sigil_build::driver::shape_defines`): a game declares comptime define
//! rows in its own placement map (`game.toml` `map`, `[defines]`), and the merge with the
//! rows its descriptions state (the shape's derived features, the engine's and the game's
//! `[defines]`, the row's overrides) is conflict-checked.
//!
//! Synthetic projects (`test_support::write_project_fixture`), plus reference-gated
//! closers against the real tree. Which rows the shipped maps declare is aeon's to decide;
//! the closers read that population from the maps rather than assuming it.
//!
//! G0b Task 11 moved this file off the profile-driven driver's `native::shape_defines`,
//! whose "built-in rows" were Rust literals in native.rs; the descriptions now state them.
//! The game-declared rows are held to the polarity property `tests/shipped_shapes.rs`
//! gives every row — see `GAME_DECLARED_EXEMPT` below.

use sigil_build::driver;
use sigil_harness::shapes;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// A synthetic project (game `g`, shape `release`) whose engine declares
/// `MAX_RING_BUFFER = 128` and whose placement map is `body`. Returns the tempdir.
fn project_with_map(body: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let game = sigil_harness::test_support::write_project_fixture(tmp.path());
    let engine = tmp.path().join("engine/engine.toml");
    let mut text = std::fs::read_to_string(&engine).unwrap();
    text.push_str("[defines]\nMAX_RING_BUFFER = 128\n");
    std::fs::write(&engine, text).unwrap();
    std::fs::write(game.join("map.toml"), body).unwrap();
    tmp
}

fn release(tmp: &tempfile::TempDir) -> Result<Vec<(String, i128)>, String> {
    shapes::shape_defines(tmp.path(), &shapes::Shape::new("g", "release"))
}

/// The rows the descriptions alone state for `release` (the merge over an empty map).
fn described(tmp: &tempfile::TempDir) -> Vec<(String, i128)> {
    let p = shapes::Shape::new("g", "release").project(tmp.path()).expect("project");
    let shape = sigil_build::describe::resolve_shape(&p.game, "release").expect("shape");
    driver::merged_defines(&p, &shape, "").expect("described rows")
}

fn as_pairs(defines: &[(String, i128)]) -> Vec<(&str, i128)> {
    defines.iter().map(|(k, v)| (k.as_str(), *v)).collect()
}

#[test]
fn a_game_declared_row_reaches_the_shape_define_env() {
    // A FRESH game key lands in the merged env with its declared value, and every
    // described row survives beside it.
    let tmp = project_with_map("[defines]\nSCANLINE_CAPS = 20\n");
    let merged = release(&tmp).expect("shape defines");
    let pairs = as_pairs(&merged);
    assert!(pairs.contains(&("SCANLINE_CAPS", 20)), "game row missing: {pairs:?}");
    let builtin = described(&tmp);
    for (k, v) in &builtin {
        assert!(pairs.contains(&(k.as_str(), *v)), "described row ({k}, {v}) lost in the merge: {pairs:?}");
    }
    assert_eq!(merged.len(), builtin.len() + 1, "{pairs:?}");
}

#[test]
fn a_game_row_shadowing_a_builtin_is_a_loud_error_naming_both_sources() {
    // A map row may not shadow a row the descriptions state (and a description may not
    // silently override the map's value). The engine declares MAX_RING_BUFFER = 128; a
    // map redeclaring it must stop the build naming the key, the map and the descriptions.
    let tmp = project_with_map("[defines]\nMAX_RING_BUFFER = 64\n");
    let err = release(&tmp).expect_err("shadowing a described row must fail");
    assert!(err.contains("MAX_RING_BUFFER"), "error must name the key: {err}");
    assert!(err.contains("map.toml"), "error must name the game-config source: {err}");
    assert!(err.contains("descriptions already declare"), "error must name the described source: {err}");
}

#[test]
fn a_duplicated_key_in_the_defines_table_is_a_loud_error() {
    let tmp = project_with_map("[defines]\nSCANLINE_CAPS = 20\nSCANLINE_CAPS = 24\n");
    let err = release(&tmp).expect_err("a duplicated key must fail");
    assert!(err.contains("SCANLINE_CAPS"), "error must name the key: {err}");
    assert!(err.contains("map.toml"), "error must name the source: {err}");
    assert!(err.contains("declared twice"), "error must say what is wrong: {err}");
}

#[test]
fn a_map_without_a_defines_table_yields_exactly_the_builtin_rows() {
    // The byte-neutral default: region/placement keys only. The merged env must equal
    // the described rows verbatim.
    let tmp = project_with_map("fill = 0x00\n[[region]]\nname = \"rom\"\nlma_base = 0\nsize = 0x400000\nkind = \"rom\"\n");
    assert_eq!(release(&tmp).expect("shape defines"), described(&tmp));
}

#[test]
fn a_tree_with_no_game_map_fails_naming_the_file() {
    // A game homes its `[defines]` rows in its placement map, so the file's ABSENCE is
    // a missing config, not an empty one: tolerating it would hand back a
    // descriptions-only env and land the symptom on the `.emp` consumer.
    let tmp = project_with_map("");
    std::fs::remove_file(tmp.path().join("games/g/map.toml")).unwrap();
    let err = release(&tmp).expect_err("a shape must not walk without its game config");
    assert!(err.contains("games/g"), "error must name the game: {err}");
    assert!(err.contains("map.toml"), "error must name the missing FILE: {err}");
}

#[test]
fn an_empty_game_map_file_is_enough_to_walk() {
    // The other arm of the same rule: a synthetic tree opts in by SUPPLYING the file.
    // An empty one declares no game rows and the merge is exactly the described rows.
    let tmp = project_with_map("");
    assert_eq!(release(&tmp).expect("shape defines"), described(&tmp));
}

#[test]
fn a_malformed_game_map_is_loud() {
    // Absence and malformation are different failures with different fixes, so the
    // malformed map keeps its own parse diagnostic naming the source.
    let tmp = project_with_map("[defines]\nBROKEN = \n");
    let err = release(&tmp).expect_err("a malformed map must fail");
    assert!(err.contains("map.toml"), "error must name the source: {err}");
    assert!(err.contains("parse error"), "error must say it is a parse failure: {err}");
}

/// The rows the descriptions alone state for a shipped shape (its merge over an empty map).
fn described_rows(aeon: &Path, s: &shapes::Shape) -> Vec<(String, i128)> {
    let p = s.project(aeon).unwrap_or_else(|e| panic!("shape `{}`: {e}", s.name()));
    let shape = sigil_build::describe::resolve_shape(&p.game, &s.shape).expect("shape");
    driver::merged_defines(&p, &shape, "").unwrap_or_else(|e| panic!("shape `{}`: {e}", s.name()))
}

fn real_tree() -> Option<std::path::PathBuf> {
    sigil_harness::test_support::reference_tree(&["games/sonic4/map.toml", "games/demo/map.toml"])
}

/// Shapes of one game (which share its placement map) carry the SAME described key set.
///
/// The described-vs-map conflict check runs per shape; if one shape of a game carried a
/// described key the others lack, a map `[defines]` row of that name would error in that
/// shape and silently land in the rest. A description cannot do it (a row only
/// overrides a key its engine or game declares, `[define.undeclared-override]`); this
/// holds the real tree to it.
#[test]
fn shapes_sharing_a_game_map_carry_the_same_builtin_key_set() {
    let Some(aeon) = real_tree() else { return };
    let mut by_game: BTreeMap<String, Vec<(String, BTreeSet<String>)>> = BTreeMap::new();
    for s in shapes::shipped(&aeon).expect("shipped shapes") {
        let keys = described_rows(&aeon, &s).into_iter().map(|(k, _)| k).collect();
        by_game.entry(s.game.clone()).or_default().push((s.name(), keys));
    }
    for (game, group) in &by_game {
        let (first_label, first_keys) = &group[0];
        for (label, keys) in &group[1..] {
            assert_eq!(
                keys, first_keys,
                "shapes `{first_label}` and `{label}` share games/{game}'s map but carry different \
                 described key sets, a map [defines] row named for the difference would error in \
                 one shape and silently land in the other"
            );
        }
    }
    assert!(by_game.len() >= 2, "expected at least the sonic4 and demo groups: {by_game:?}");
}

/// Every shipped shape's merge SUCCEEDS against the real aeon tree, and every described
/// row survives it: a shipped map whose `[defines]` collides with a described row fails
/// here (and in every build) the moment it lands. Reference-gated: skips green when the
/// tree is absent, panics under `SIGIL_STRICT_GATE=1`.
#[test]
fn every_shipped_shape_merges_cleanly_against_the_real_tree() {
    let Some(aeon) = real_tree() else { return };
    for s in shapes::shipped(&aeon).expect("shipped shapes") {
        let label = s.name();
        let merged = shapes::shape_defines(&aeon, &s).unwrap_or_else(|e| panic!("shape `{label}`: {e}"));
        let pairs = as_pairs(&merged);
        for (k, v) in &described_rows(&aeon, &s) {
            assert!(
                pairs.contains(&(k.as_str(), *v)),
                "shape `{label}`: described row ({k}, {v}) lost in the merge: {pairs:?}"
            );
        }
    }
}

// ── The game-declared half of the polarity net ────────────────────────────────
//
// `audit_game_declared_polarity` charges the map-declared rows the both-polarities
// property, and a row that cannot meet it must be NAMED here.

/// Game-declared defines whose blind arm is an accepted, reasoned decision. The audit
/// rejects a reason-less entry and an entry no shape declares.
const GAME_DECLARED_EXEMPT: &[(&str, &str)] = &[];

/// The audit over one tree: every shipped shape's merged env, minus the described keys.
fn audit_tree(aeon: &Path) -> Result<BTreeSet<String>, String> {
    let mut builtin_keys: BTreeSet<String> = BTreeSet::new();
    let mut named: Vec<(String, Vec<(String, i128)>)> = Vec::new();
    for s in shapes::shipped(aeon).expect("shipped shapes") {
        builtin_keys.extend(described_rows(aeon, &s).into_iter().map(|(k, _)| k));
        let merged = shapes::shape_defines(aeon, &s).unwrap_or_else(|e| panic!("shape `{}`: {e}", s.name()));
        named.push((s.name(), merged));
    }
    let envs: Vec<(&str, Vec<(String, i128)>)> = named.iter().map(|(l, e)| (l.as_str(), e.clone())).collect();
    sigil_harness::game_defines::audit_game_declared_polarity(&envs, &builtin_keys, GAME_DECLARED_EXEMPT)
}

/// A synthetic two-game project (`g` and `h`, each with a `release` and a `debug`
/// shape) whose maps are `g_map` and `h_map`.
fn two_game_tree(g_map: &str, h_map: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let g = sigil_harness::test_support::write_project_fixture(tmp.path());
    let debug = "[shapes.debug]\nfeatures = { debug = true, crash_report = true, sound = false }\n";
    let gt = std::fs::read_to_string(g.join("game.toml")).unwrap() + debug;
    std::fs::write(g.join("game.toml"), &gt).unwrap();
    let h = tmp.path().join("games/h");
    std::fs::create_dir_all(&h).unwrap();
    std::fs::write(h.join("game.toml"), gt.replace("id = \"g\"", "id = \"h\"").replace("games.g.", "games.h.")).unwrap();
    std::fs::write(h.join("baseline.toml"), "format = 1\n").unwrap();
    std::fs::write(g.join("map.toml"), g_map).unwrap();
    std::fs::write(h.join("map.toml"), h_map).unwrap();
    tmp
}

#[test]
fn a_game_declared_toggle_row_is_caught_by_the_polarity_audit() {
    // POISON, through the real seam: g's map declares a 0/1 row. Both of g's shapes see
    // `1` and h declares nothing, so the `0` arm of any comptime `if` on it is
    // unreachable by every shape — named.
    let tmp = two_game_tree("[defines]\nFANCY_ARM = 1\n", "");
    let err = audit_tree(tmp.path()).expect_err("a pinned game-declared toggle must be named");
    assert!(err.contains("FANCY_ARM"), "audit must name the key: {err}");
    assert!(err.contains("TOGGLE"), "audit must say it is a toggle: {err}");
    assert!(err.contains("g release") || err.contains("g debug"), "audit must name a declaring shape: {err}");
}

#[test]
fn a_game_declared_toggle_walked_in_both_polarities_is_accepted() {
    // CONTROL: the two games declare opposite values, so both arms are walked.
    let tmp = two_game_tree("[defines]\nFANCY_ARM = 1\n", "[defines]\nFANCY_ARM = 0\n");
    let keys = audit_tree(tmp.path()).expect("a both-polarity game toggle is exactly the goal");
    assert!(keys.contains("FANCY_ARM"), "the audit must have SEEN the row: {keys:?}");
}

/// The `[defines]` keys the shipped maps declare, read by a route that shares nothing
/// with the subject: `std::fs` and the `toml` crate's document model. The SET of maps is
/// every distinct placement map a shipped shape reads (`Project::map_path`).
fn declared_game_define_keys(aeon: &Path) -> BTreeSet<String> {
    let maps: BTreeSet<std::path::PathBuf> = shapes::shipped(aeon)
        .expect("shipped shapes")
        .iter()
        .map(|s| s.map_path(aeon).expect("map path"))
        .collect();
    let mut keys = BTreeSet::new();
    for map in &maps {
        let src = std::fs::read_to_string(map).unwrap_or_else(|e| panic!("read {}: {e}", map.display()));
        let doc: toml::Table =
            toml::from_str(&src).unwrap_or_else(|e| panic!("{}: not a TOML document: {e}", map.display()));
        match doc.get("defines") {
            None => {}
            Some(toml::Value::Table(rows)) => keys.extend(rows.keys().cloned()),
            Some(other) => panic!("{}: `defines` is a {}, not a table", map.display(), other.type_str()),
        }
    }
    keys
}

/// The audit's judged key set, held EQUAL to the keys the maps declare (both directions).
fn assert_audit_judged_every_declared_key(aeon: &Path) -> BTreeSet<String> {
    let judged = audit_tree(aeon).unwrap_or_else(|e| panic!("{e}"));
    let declared = declared_game_define_keys(aeon);
    let unjudged: Vec<&String> = declared.difference(&judged).collect();
    let undeclared: Vec<&String> = judged.difference(&declared).collect();
    assert!(
        unjudged.is_empty() && undeclared.is_empty(),
        "the polarity audit must judge exactly the game [defines] keys the shipped maps \
         declare: declared but not judged {unjudged:?}, judged but not declared \
         {undeclared:?} (the maps declare {declared:?}, the audit judged {judged:?})"
    );
    judged
}

#[test]
fn a_planted_game_row_is_judged_exactly_as_the_maps_declare_it() {
    // CONTROL for the derived expectation, with no aeon tree: both maps declare a toggle
    // walked in both polarities and a value walked at two sizes.
    let tmp = two_game_tree("[defines]\nFANCY_ARM = 1\nBAND_CAPS = 0x0FDE\n", "[defines]\nFANCY_ARM = 0\nBAND_CAPS = 0\n");
    let judged = assert_audit_judged_every_declared_key(tmp.path());
    let planted: BTreeSet<String> = ["BAND_CAPS", "FANCY_ARM"].map(String::from).into();
    assert_eq!(judged, planted, "the audit must have judged both planted rows");
}

/// The real maps: the audit runs over the shipped tree and judges exactly the rows
/// those maps declare. Reference-gated.
#[test]
fn the_shipped_maps_game_declared_rows_are_polarity_covered() {
    let Some(aeon) = real_tree() else { return };
    assert_audit_judged_every_declared_key(&aeon);
}
