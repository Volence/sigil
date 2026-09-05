//! The per-game `[defines]` mechanism, end to end through `native::shape_defines`:
//! a game declares comptime define rows in its own `games/<g>/map.toml`, and the
//! merge with the profile's built-in rows is conflict-checked in both directions.
//!
//! Synthetic trees, plus reference-gated closers against the real maps.
//! `shape_defines` reads nothing but the game's map.toml, so a tempdir holding
//! just `games/sonic4/map.toml` is a complete fixture — no aeon checkout is
//! written, and no shipped map declares a row (the byte gates hold that neutral
//! state; adoption is the aeon-owned paired lane).
//!
//! Two properties beyond the merge itself live here. A shipped shape whose map is
//! ABSENT fails naming the file (a synthetic fixture opts out explicitly), and the
//! game-declared rows are held to the polarity property the built-in rows get from
//! `tests/shipped_shapes.rs` — see `GAME_DECLARED_EXEMPT` below.

use sigil_harness::native;
use std::path::Path;

/// A tempdir tree whose `games/sonic4/map.toml` is `body` — the sonic4 profile's
/// `map_path` resolves there, so this is the whole game-config surface.
fn tree_with_sonic4_map(body: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let map = tmp.path().join("games/sonic4/map.toml");
    std::fs::create_dir_all(map.parent().unwrap()).expect("mkdir games/sonic4");
    std::fs::write(&map, body).expect("write map.toml");
    tmp
}

fn as_pairs(defines: &[(String, i128)]) -> Vec<(&str, i128)> {
    defines.iter().map(|(k, v)| (k.as_str(), *v)).collect()
}

#[test]
fn a_game_declared_row_reaches_the_shape_define_env() {
    // Precedence direction 1: a FRESH game key lands in the merged env with its
    // declared value, and every built-in row survives beside it.
    let tmp = tree_with_sonic4_map("[defines]\nSCANLINE_CAPS = 20\n");
    let profile = native::sonic4_profile(false);
    let merged = native::shape_defines(&profile, tmp.path()).expect("shape defines");

    let pairs = as_pairs(&merged);
    assert!(pairs.contains(&("SCANLINE_CAPS", 20)), "game row missing: {pairs:?}");
    for (k, v) in &profile.emp_defines {
        assert!(
            pairs.contains(&(*k, *v)),
            "built-in row ({k}, {v}) lost in the merge: {pairs:?}"
        );
    }
    assert_eq!(merged.len(), profile.emp_defines.len() + 1, "{pairs:?}");
}

#[test]
fn a_game_row_shadowing_a_builtin_is_a_loud_error_naming_both_sources() {
    // Precedence direction 2: a game row may not shadow a built-in row (and the
    // built-in may not silently override the game's declared value). sonic4's
    // built-in MAX_RING_BUFFER is 128; a map redeclaring it must stop the build
    // naming the key, the map.toml, and the built-in home.
    let tmp = tree_with_sonic4_map("[defines]\nMAX_RING_BUFFER = 64\n");
    let profile = native::sonic4_profile(false);
    let err = native::shape_defines(&profile, tmp.path())
        .expect_err("shadowing a built-in must fail");
    assert!(err.contains("MAX_RING_BUFFER"), "error must name the key: {err}");
    assert!(err.contains("map.toml"), "error must name the game-config source: {err}");
    assert!(err.contains("native.rs"), "error must name the built-in source: {err}");
    assert!(err.contains("= 64") && err.contains("= 128"), "error names both values: {err}");
}

#[test]
fn a_duplicated_key_in_the_defines_table_is_a_loud_error() {
    let tmp = tree_with_sonic4_map("[defines]\nSCANLINE_CAPS = 20\nSCANLINE_CAPS = 24\n");
    let profile = native::sonic4_profile(false);
    let err = native::shape_defines(&profile, tmp.path())
        .expect_err("a duplicated key must fail");
    assert!(err.contains("SCANLINE_CAPS"), "error must name the key: {err}");
    assert!(err.contains("map.toml"), "error must name the source: {err}");
    assert!(err.contains("declared twice"), "error must say what is wrong: {err}");
}

#[test]
fn a_map_without_a_defines_table_yields_exactly_the_builtin_rows() {
    // The byte-neutral default every shipped map has today: region/placement
    // keys only. The merged env must equal the built-ins verbatim.
    let tmp = tree_with_sonic4_map(
        "fill = 0x00\n[[region]]\nname = \"rom\"\nlma_base = 0\nsize = 0x400000\nkind = \"rom\"\n",
    );
    let profile = native::sonic4_profile(false);
    let merged = native::shape_defines(&profile, tmp.path()).expect("shape defines");
    let builtin: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    assert_eq!(merged, builtin);
}

#[test]
fn a_tree_with_no_game_map_fails_naming_the_file() {
    // A game homes its `[defines]` rows in `games/<g>/map.toml`, so the file's
    // ABSENCE is a missing config, not an empty one. Tolerating it would hand
    // back a built-ins-only env and let the shape walk define-complete but
    // game-row-free — with the eventual symptom landing on the `.emp` consumer
    // instead of on the file that went missing.
    let tmp = tempfile::tempdir().expect("tempdir");
    let profile = native::sonic4_profile(false);
    let err = native::shape_defines(&profile, tmp.path())
        .expect_err("a shape must not walk without its game config");
    assert!(err.contains("sonic4"), "error must name the shape: {err}");
    assert!(err.contains("map.toml"), "error must name the missing FILE: {err}");
    assert!(
        err.contains("supplies the file"),
        "error must tell a synthetic tree what to do about it: {err}"
    );
}

#[test]
fn an_empty_game_map_file_is_enough_to_walk() {
    // The other arm of the same rule, and the one a refuse-everything
    // implementation fails: there is no waiver to grant, so a synthetic tree
    // opts in by SUPPLYING the file. An empty one declares no game rows and the
    // merge is exactly the built-ins.
    let tmp = tree_with_sonic4_map("");
    let profile = native::sonic4_profile(false);
    let merged = native::shape_defines(&profile, tmp.path()).expect("shape defines");
    let builtin: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    assert_eq!(merged, builtin);
}

#[test]
fn a_malformed_game_map_is_loud() {
    // Absence and malformation are different failures with different fixes, so
    // the malformed map keeps its own parse diagnostic naming the source.
    let tmp = tree_with_sonic4_map("[defines]\nBROKEN = \n");
    let profile = native::sonic4_profile(false);
    let err = native::shape_defines(&profile, tmp.path())
        .expect_err("a malformed map must fail");
    assert!(err.contains("map.toml"), "error must name the source: {err}");
    assert!(err.contains("parse error"), "error must say it is a parse failure: {err}");
}

/// Shapes sharing a `games/<g>/map.toml` carry the SAME built-in key set.
///
/// The built-in-vs-game conflict check runs per shape; if one map-sharing shape
/// carried a built-in key the others lack, a game `[defines]` row of that name
/// would error in that shape and silently land in the rest — the same map read
/// two ways. Key-set equality across each map-path group makes that split
/// unrepresentable. Reference-free: `map_path` only joins path components, so
/// the grouping needs no tree on disk.
#[test]
fn shapes_sharing_a_game_map_carry_the_same_builtin_key_set() {
    use std::collections::{BTreeMap, BTreeSet};

    let anchor = Path::new("/");
    let mut by_map: BTreeMap<std::path::PathBuf, Vec<(&str, BTreeSet<&str>)>> = BTreeMap::new();
    for (label, profile) in native::shipped_shapes() {
        let keys: BTreeSet<&str> = profile.emp_defines.iter().map(|(k, _)| *k).collect();
        by_map.entry(profile.map_path(anchor)).or_default().push((label, keys));
    }
    for (map, group) in &by_map {
        let (first_label, first_keys) = &group[0];
        for (label, keys) in &group[1..] {
            assert_eq!(
                keys,
                first_keys,
                "shapes `{first_label}` and `{label}` share {} but carry different \
                 built-in key sets, a game [defines] row named for the difference \
                 would error in one shape and silently land in the other",
                map.display()
            );
        }
    }
    assert!(by_map.len() >= 2, "expected at least the sonic4 and demo map groups: {by_map:?}");
}

/// Every shipped shape's merge SUCCEEDS against the real aeon tree, and every
/// built-in row survives it. This is the consumer-side half of the drift net:
/// a shipped map whose `[defines]` table collides with a built-in row fails
/// here (and in every build) the moment it lands, and a game row can only
/// EXTEND the env, never replace a built-in. Reference-gated through the
/// canonical seam: skips green when the tree is absent, panics under
/// `SIGIL_STRICT_GATE=1`.
#[test]
fn every_shipped_shape_merges_cleanly_against_the_real_tree() {
    let Some(aeon) = sigil_harness::test_support::reference_tree(&[
        "games/sonic4/map.toml",
        "games/demo/map.toml",
    ]) else {
        return;
    };
    for (label, profile) in native::shipped_shapes() {
        let merged = shape_or_panic(&profile, &aeon, label);
        let pairs = as_pairs(&merged);
        for (k, v) in &profile.emp_defines {
            assert!(
                pairs.contains(&(*k, *v)),
                "shape `{label}`: built-in row ({k}, {v}) lost in the merge: {pairs:?}"
            );
        }
    }
}

fn shape_or_panic(
    profile: &native::GameProfile,
    aeon: &Path,
    label: &str,
) -> Vec<(String, i128)> {
    native::shape_defines(profile, aeon)
        .unwrap_or_else(|e| panic!("shape `{label}`: shape_defines: {e}"))
}

// ── The game-declared half of the polarity net ────────────────────────────────
//
// `tests/shipped_shapes.rs`'s both-polarities gate iterates `profile.emp_defines`
// — the BUILT-IN rows. A game-declared row is not in that walk, so without the
// audit below it would be silently absent from the net: pinned to one value
// across every shape, a comptime `if` branching on it would carry an arm no shape
// walk reaches. `audit_game_declared_polarity` charges the game rows the same
// property, and a row that cannot meet it must be NAMED here.

/// Game-declared defines whose blind arm is an accepted, reasoned decision.
///
/// EMPTY: no shipped `games/<g>/map.toml` declares a `[defines]` row today. A row
/// that lands and cannot be walked in both polarities is named here with the
/// reason its other arm needs no coverage — the audit rejects a reason-less row
/// and rejects an entry no shape declares, so this list can neither be a blanket
/// nor outlive its subject.
const GAME_DECLARED_EXEMPT: &[(&str, &str)] = &[];

/// The audit over one tree: every shipped shape's merged env, minus the built-in
/// keys the other gate already walks.
fn audit_tree(aeon: &Path) -> Result<std::collections::BTreeSet<String>, String> {
    use std::collections::BTreeSet;

    let shapes = native::shipped_shapes();
    let mut builtin_keys: BTreeSet<String> = BTreeSet::new();
    let mut envs: Vec<(&str, Vec<(String, i128)>)> = Vec::new();
    for (label, profile) in &shapes {
        for (k, _) in &profile.emp_defines {
            builtin_keys.insert((*k).to_string());
        }
        envs.push((*label, shape_or_panic(profile, aeon, label)));
    }
    sigil_harness::game_defines::audit_game_declared_polarity(
        &envs,
        &builtin_keys,
        GAME_DECLARED_EXEMPT,
    )
}

/// A synthetic tree carrying both shipped maps, so all seven shapes resolve.
fn tree_with_both_maps(sonic4: &str, demo: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    for (game, body) in [("sonic4", sonic4), ("demo", demo)] {
        let map = tmp.path().join(format!("games/{game}/map.toml"));
        std::fs::create_dir_all(map.parent().unwrap()).expect("mkdir");
        std::fs::write(&map, body).expect("write map.toml");
    }
    tmp
}

#[test]
fn a_game_declared_toggle_row_is_caught_by_the_polarity_audit() {
    // POISON, through the real seam: sonic4's map declares a 0/1 row. Five of the
    // seven shapes read that map and all see `1`, so the `0` arm of any comptime
    // `if` on it is unreachable by every shape — the latent, now loud and named.
    let tmp = tree_with_both_maps("[defines]\nFANCY_ARM = 1\n", "");
    let err = audit_tree(tmp.path()).expect_err("a pinned game-declared toggle must be named");
    assert!(err.contains("FANCY_ARM"), "audit must name the key: {err}");
    assert!(err.contains("TOGGLE"), "audit must say it is a toggle: {err}");
    assert!(err.contains("sonic4 plain"), "audit must name a declaring shape: {err}");
}

#[test]
fn a_game_declared_toggle_walked_in_both_polarities_is_accepted() {
    // CONTROL, through the same seam: the two games declare opposite values, so
    // both arms are walked and the audit must pass. This is the arm an audit that
    // simply refused every game row would fail.
    let tmp = tree_with_both_maps("[defines]\nFANCY_ARM = 1\n", "[defines]\nFANCY_ARM = 0\n");
    let keys = audit_tree(tmp.path()).expect("a both-polarity game toggle is exactly the goal");
    assert!(keys.contains("FANCY_ARM"), "the audit must have SEEN the row: {keys:?}");
}

/// The real maps: the audit runs over the shipped tree, not only over fixtures.
/// Reference-gated through the canonical seam — skips green when the tree is
/// absent, panics under `SIGIL_STRICT_GATE=1`.
#[test]
fn the_shipped_maps_game_declared_rows_are_polarity_covered() {
    let Some(aeon) = sigil_harness::test_support::reference_tree(&[
        "games/sonic4/map.toml",
        "games/demo/map.toml",
    ]) else {
        return;
    };
    let keys = audit_tree(&aeon).unwrap_or_else(|e| panic!("{e}"));
    // Today's state, asserted rather than assumed: no shipped map declares a row,
    // so the audit walked an empty residue. When the first row lands this line is
    // the one that changes, in the same commit as the row.
    assert!(
        keys.is_empty(),
        "the shipped maps now declare game defines {keys:?}, update this expectation \
         in the same commit as the row"
    );
}
