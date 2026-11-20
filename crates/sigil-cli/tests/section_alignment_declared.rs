//! R7 — EVERY SHIPPED SHAPE SATISFIES ITS SECTIONS' DECLARED ALIGNMENT, and every
//! requirement a window wide is held by an anchor that satisfies it.
//!
//! The alignment each ROM section REQUIRES beyond "even" is declared in the descriptions:
//! the engine's `[[align]]` rows (`engine/engine.toml`) and the game's (`games/<g>/game.toml`).
//! The game-as-data driver (`sigil build --project`) rounds every chained section's base to
//! its declaration and nothing else, and checks each resolved base against it after the
//! resolve (the independent instrument for the sections it places by another rule: declared
//! `[[anchor]]` islands, phase-bank orgs, label-less contiguity blobs). `seam2::sound_layout`
//! predicts the two sound blobs' bases through the same rounding, and the fold check holds
//! the prediction to the placement.
//!
//! G0b Task 11 moved this file off the profile-driven driver, whose row table
//! (`section_align::DECLARED`) and frozen size tables are deleted. Its witness that a frozen
//! row's residue was not an alignment input (`a_doctored_pin_residue_does_not_move_a_packed_section`)
//! retired with that driver: the game-as-data driver reads no frozen table, so there is no
//! input left to doctor.
//!
//! Reference tree: `AEON_DIR`, else the sibling aeon checkout. A missing tree skips
//! green outside strict mode and HARD-FAILS naming the absent path under
//! `SIGIL_STRICT_GATE=1`.

use sigil_build::describe::AlignRow;
use sigil_harness::map_placement::load_placement_map;
use sigil_harness::shapes::{self, Shape};
use sigil_harness::test_support::{reference_tree, reference_tree_for_shape};
use std::path::Path;

/// Every `[[align]]` row the shape's build reads: the engine's, then its game's.
fn declared(aeon: &Path, s: &Shape) -> Vec<AlignRow> {
    let p = s.project(aeon).unwrap_or_else(|e| panic!("{}: {e}", s.name()));
    p.engine.align.iter().chain(p.game.align.iter()).cloned().collect()
}

/// THE GATE: every shipped shape builds, which is exactly "every shipped shape's every
/// section satisfies its declared alignment, and seam2's prediction agrees with the
/// walk's placement": all three checks are always-on inside the build, so a shape that
/// violated one could not return `Ok`.
#[test]
fn every_shipped_shape_satisfies_its_declared_alignment() {
    let Some(aeon) = reference_tree(&["engine/engine.toml"]) else { return };
    let shipped = shapes::shipped(&aeon).expect("shipped shapes");
    assert!(!shipped.is_empty(), "shapes::shipped enumerated nothing, a gate over no shapes gates nothing");
    for s in &shipped {
        let Some(_) = reference_tree_for_shape(s) else { return };
    }

    // Measure every shape before judging any, so a run names EVERY violator.
    let faults: Vec<String> = shipped
        .iter()
        .filter_map(|s| shapes::build(&aeon, s).err().map(|e| format!("shape `{}`: {e}", s.name())))
        .collect();
    assert!(
        faults.is_empty(),
        "{} of {} shipped shapes fail a placement check (an alignment fault reads \
         `[layout.alignment-violated]`; a prediction fault reads `[sound.fold-vs-placement]`):\n  {}",
        faults.len(),
        shipped.len(),
        faults.join("\n  ")
    );
    let rows: usize = shipped.iter().map(|s| declared(&aeon, s).len()).max().unwrap_or(0);
    assert!(rows > 0, "no shipped shape declares an `[[align]]` row, the gate measures only evenness");
    eprintln!(
        "declared alignment: {} shipped shapes green, up to {rows} declared sections per shape, at {}",
        shipped.len(),
        aeon.display()
    );
}

/// The rows above 16 the walk HOLDS: a map `[[anchor]]` whose `holds` names the section
/// places it absolute, and only the post-resolve check measures it against the
/// declaration.
const HELD_ABOVE_16: &[(&str, u32)] = &[
    ("Dac_Temp_Blip", 0x8000),
    ("SoundTablesZ80_Head", 0x8000),
    ("SongBank2_Head", 0x8000),
    ("ObjCodeBase", 0x10000),
];

/// The rows above 16 the walk ROUNDS: no anchor holds them, so the running cursor is
/// rounded up to the declaration. Empty since the game-as-data driver: `SongBank2_Head`
/// was rounded under the profile-driven driver (no frozen row named it) and is held by
/// its `song_bank_2` anchor now.
const ROUNDED_ABOVE_16: &[(&str, u32)] = &[];

/// THE REQUIREMENTS ABOVE 16, as declared, and the rule that places each. The Z80 bank
/// heads require `$8000` (one `SetBank` window, `bankid() = (lma & $7F8000) >> 15`) and
/// `ObjCodeBase` requires `$10000` (aeon's R1 ruling). Every row above 16 is either HELD
/// (an anchor `holds` it, at an address that satisfies the requirement) or ROUNDED (no
/// anchor holds it). The partition is read off the descriptions and the sonic4 map, so a
/// new row above 16 fails here until it is classified, and a held row whose anchor stops
/// satisfying its requirement fails naming both.
#[test]
fn the_requirements_above_16_are_declared_held_or_rounded() {
    let s = Shape::new("sonic4", "release");
    let Some(aeon) = reference_tree_for_shape(&s) else { return };
    let rows = declared(&aeon, &s);
    let required_for = |label: &str| rows.iter().find(|r| r.label == label).map(|r| r.align);
    for (label, want) in HELD_ABOVE_16.iter().chain(ROUNDED_ABOVE_16) {
        let got = required_for(label).unwrap_or_else(|| panic!("`{label}` must be declared"));
        assert_eq!(got, *want, "`{label}`'s declared requirement");
    }

    let classified: Vec<&str> = HELD_ABOVE_16.iter().chain(ROUNDED_ABOVE_16).map(|(l, _)| *l).collect();
    let unclassified: Vec<&str> =
        rows.iter().filter(|r| r.align > 16 && !classified.contains(&r.label.as_str())).map(|r| r.label.as_str()).collect();
    assert!(unclassified.is_empty(), "rows above 16 classified neither HELD nor ROUNDED: {unclassified:?}");

    let map_path = s.map_path(&aeon).expect("map path");
    let map_src = std::fs::read_to_string(&map_path).unwrap_or_else(|e| panic!("read {}: {e}", map_path.display()));
    let pmap = load_placement_map(&map_src).unwrap_or_else(|e| panic!("{}: {e}", map_path.display()));
    let held_by = |label: &str| -> Vec<(String, u32)> {
        pmap.anchors
            .iter()
            .filter(|a| a.holds.as_deref() == Some(label))
            .map(|a| (a.name.clone(), a.at))
            .collect()
    };
    for (label, want) in HELD_ABOVE_16 {
        let anchors = held_by(label);
        assert!(
            !anchors.is_empty(),
            "HELD `{label}` is held by no anchor in {}, so the walk would round it instead of \
             holding it",
            map_path.display()
        );
        for (anchor, at) in anchors {
            assert!(
                at.is_multiple_of(*want),
                "HELD `{label}`'s anchor `{anchor}` is at {at:#x}, not a multiple of {want:#x}"
            );
        }
    }
    for (label, _) in ROUNDED_ABOVE_16 {
        let anchors = held_by(label);
        assert!(
            anchors.is_empty(),
            "ROUNDED `{label}` is held by anchor(s) {anchors:?}; the walk places it there rather \
             than rounding it"
        );
    }
}
