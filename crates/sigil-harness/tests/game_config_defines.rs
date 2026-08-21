//! The per-game `[defines]` mechanism, end to end through `native::shape_defines`:
//! a game declares comptime define rows in its own `games/<g>/map.toml`, and the
//! merge with the profile's built-in rows is conflict-checked in both directions.
//!
//! Synthetic trees, plus one reference-gated closer against the real maps.
//! `shape_defines` reads nothing but the game's map.toml, so a tempdir holding
//! just `games/sonic4/map.toml` is a complete fixture — no aeon checkout is
//! written, and no shipped map declares a row (the byte gates hold that neutral
//! state; adoption is the aeon-owned paired lane).

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
fn a_tree_with_no_game_map_declares_no_game_rows() {
    // The port fixtures build synthetic trees with no games/<g>/map.toml; a
    // shape with no map declares no game defines and the merge is the built-ins.
    let tmp = tempfile::tempdir().expect("tempdir");
    let profile = native::sonic4_profile(false);
    let merged = native::shape_defines(&profile, tmp.path()).expect("shape defines");
    let builtin: Vec<(String, i128)> =
        profile.emp_defines.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    assert_eq!(merged, builtin);
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
