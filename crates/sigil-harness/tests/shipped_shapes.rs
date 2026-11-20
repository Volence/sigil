//! The shape SET itself, pinned.
//!
//! [`shapes::shipped`] is what every gate meaning "all shipped shapes" walks: every
//! non-fixture `[shapes.*]` row of every `games/*/game.toml`.
//! Adding a row to it is loud — `warn_tier_corpus` panics on a shape with no baseline
//! row, and the corpus gates assert per shape. REMOVING a row is silent: the gates
//! simply walk one fewer corpus and every assertion still passes. That asymmetry is
//! the hole this file closes.
//!
//! It matters because the coverage is thin in exactly two places. `config_a` is the
//! ONLY shape carrying `SOUND_DEBUG_HOTKEYS = 1` and `SOUND_DBG_MIRROR = 1`, and
//! `lean` is the ONLY shape carrying `CRASH_REPORT = 0`. Dropping either row silently
//! reinstates a permanently blind comptime arm — `vblank.emp`'s mirror block,
//! `game.emp`'s hotkey block, `vectors.emp`'s four release-fault cells — with every
//! test in the workspace still green.
//!
//! Reference-dependent since G0b Task 11: the shape set is DATA in the aeon tree's game
//! descriptions (it was the profile-driven driver's Rust literals), so this reads the
//! tree, skips without one and panics without one under `SIGIL_STRICT_GATE`.

use sigil_harness::shapes;
use std::collections::BTreeSet;

/// The seven shipped shapes, as `<game> <row>` in [`shapes::shipped`] order (games by
/// directory name, rows by name). These are the names the `--report` header prints;
/// the golden BLOBS are named by filename (`s4.bin`, `config_a.bin`), which is a
/// different naming and deliberately so.
const EXPECTED: [&str; 7] = [
    "demo debug",
    "demo release",
    "sonic4 config-a",
    "sonic4 config-b",
    "sonic4 debug",
    "sonic4 lean",
    "sonic4 release",
];

fn tree() -> Option<std::path::PathBuf> {
    sigil_harness::test_support::reference_tree(&["games/sonic4/game.toml", "games/demo/game.toml"])
}

#[test]
fn the_shipped_shape_set_is_the_seven_the_byte_bar_builds() {
    let Some(aeon) = tree() else { return };
    let names: Vec<String> = shapes::shipped(&aeon).expect("shipped shapes").iter().map(|s| s.name()).collect();
    let got: Vec<&str> = names.iter().map(String::as_str).collect();
    assert_eq!(
        got,
        EXPECTED.to_vec(),
        "the shipped-shape set moved. ADDING a shape is already loud elsewhere; \
         REMOVING one narrows every corpus gate with no other signal, so it is pinned \
         here. Update deliberately, in the same commit as the byte bar."
    );
    let unique: BTreeSet<&str> = got.iter().copied().collect();
    assert_eq!(unique.len(), got.len(), "duplicate shape label: {got:?}");
}

/// Every comptime toggle the shapes carry is exercised in BOTH polarities.
///
/// This is the property that makes "the firing set is empty in every shape" a
/// statement about the corpus rather than about whichever arms one define set
/// happens to keep. A toggle pinned to one value across all seven shapes has an arm
/// no shape walk can reach, and no corpus gate can see into it — so the day a profile
/// edit collapses one, this fails and names it instead of leaving a silent blind arm.
///
/// SCOPE: the shape's full merged comptime `-D` set ([`shapes::shape_defines`], exactly
/// what its build lowers under: the engine's and game's described defines and the
/// row's features). The GAME-DECLARED half is also held to the same property by
/// `game_defines::audit_game_declared_polarity` in `tests/game_config_defines.rs`.
#[test]
fn every_comptime_toggle_is_walked_in_both_polarities() {
    let Some(aeon) = tree() else { return };
    let mut seen: std::collections::BTreeMap<String, BTreeSet<i128>> = Default::default();
    for s in shapes::shipped(&aeon).expect("shipped shapes") {
        for (k, v) in shapes::shape_defines(&aeon, &s).expect("shape defines") {
            seen.entry(k).or_default().insert(v);
        }
    }

    // The five BOOLEAN toggles a comptime `if` in the corpus branches on. The three
    // remaining defines (MAX_RING_BUFFER / VRAM_RING_PLACEHOLDER /
    // COLLECTED_WINDOW_SLOTS) are VALUES, not toggles — they size data rather than
    // select an arm — so they are checked for plurality, not for {0,1}.
    for toggle in [
        "SOUND_DRIVER_ENABLED",
        "DEBUG",
        "CRASH_REPORT",
        "SOUND_DEBUG_HOTKEYS",
        "SOUND_DBG_MIRROR",
    ] {
        let vals = seen
            .get(toggle)
            .unwrap_or_else(|| panic!("no shipped shape defines `{toggle}`"));
        assert!(
            vals.contains(&0) && vals.contains(&1),
            "`{toggle}` takes only {vals:?} across the shipped shapes, so one of its \
             comptime arms is unreachable by ANY shape walk and no corpus gate can see \
             into it. Either a shape was dropped, or a description changed."
        );
    }

    for value in ["MAX_RING_BUFFER", "VRAM_RING_PLACEHOLDER", "COLLECTED_WINDOW_SLOTS"] {
        let vals = seen
            .get(value)
            .unwrap_or_else(|| panic!("no shipped shape defines `{value}`"));
        assert!(
            vals.len() >= 2,
            "`{value}` is {vals:?} in every shipped shape, the game-config axis \
             collapsed, so a size-dependent lowering is walked in one shape only"
        );
    }
}
