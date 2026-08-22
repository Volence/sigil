//! The shape SET itself, pinned.
//!
//! [`native::shipped_shapes`] is what every gate meaning "all shipped shapes" walks.
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
//! Reference-free: this reads profile literals, not the aeon tree, so it runs
//! everywhere.

use sigil_harness::native;
use std::collections::BTreeSet;

/// The seven shipped shapes, in `capture_goldens.sh` build order. These labels are
/// the ones the `--report` header prints; the golden BLOBS are named by filename
/// (`s4.bin`, `config_a.bin`), which is a different naming and deliberately so.
const EXPECTED: [&str; 7] = [
    "sonic4 plain",
    "sonic4 debug",
    "demo plain",
    "demo debug",
    "config_a",
    "config_b",
    "lean",
];

#[test]
fn the_shipped_shape_set_is_the_seven_the_byte_bar_builds() {
    let got: Vec<&str> = native::shipped_shapes().into_iter().map(|(l, _)| l).collect();
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
/// SCOPE: `profile.emp_defines` is the BUILT-IN rows. A shape's full comptime `-D`
/// set is `native::shape_defines` — the built-ins merged with the game's own
/// `games/<g>/map.toml [defines]` rows — and the GAME-DECLARED half is held to the
/// same property by `game_defines::audit_game_declared_polarity`, walked over both
/// synthetic maps and the real tree in `tests/game_config_defines.rs`. This gate
/// stays reference-free (profile literals, no tree) and that gate reads the maps;
/// between them no row of either origin is unwatched.
#[test]
fn every_comptime_toggle_is_walked_in_both_polarities() {
    let shapes = native::shipped_shapes();
    let mut seen: std::collections::BTreeMap<String, BTreeSet<i128>> = Default::default();
    for (_, profile) in &shapes {
        for (k, v) in &profile.emp_defines {
            seen.entry(k.to_string()).or_default().insert(*v);
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
             into it. Either a shape was dropped, or a profile changed."
        );
    }

    for value in ["MAX_RING_BUFFER", "VRAM_RING_PLACEHOLDER", "COLLECTED_WINDOW_SLOTS"] {
        let vals = seen
            .get(value)
            .unwrap_or_else(|| panic!("no shipped shape defines `{value}`"));
        assert!(
            vals.len() >= 2,
            "`{value}` is {vals:?} in every shipped shape — the game-config axis \
             collapsed, so a size-dependent lowering is walked in one shape only"
        );
    }
}
