//! THE COMMAND-LINE DEFINES REACH THE LISTING, AND THE EQUATE POPULATION IS
//! CONTAINED SHAPE-TO-SHAPE.
//!
//! Two properties over one artifact, so the shapes are built once per test binary
//! rather than once per file.
//!
//! # 1. Every shipped shape publishes its own define env
//!
//! A value the build SUPPLIES (`native::shape_defines` — the profile's built-in
//! `emp_defines` rows merged with the game's `map.toml [defines]`) lives in no source
//! file the resolver walks, so before this parcel it had no listing row at all. It is
//! also exactly the class of value a reader cannot derive from the ROM:
//! `MAX_RING_BUFFER` is 128 for sonic4 and 16 for demo, so a debugger panel wanting
//! the ring-buffer ceiling had to hardcode one of the two and be wrong for the other
//! BY CONSTRUCTION. This walks every shipped shape and charges its listing with
//! carrying every row of its own define env, at that shape's own value.
//!
//! The expectation is DERIVED, not transcribed: it is `shape_defines(profile, aeon)`
//! read from the profile and the game's map, the same call the build's lowering
//! makes. A hand-written list of expected names would be a second record of one fact
//! and would go stale the first time a game declared a row.
//!
//! # 2. Release ⊆ debug, as SET CONTAINMENT, in both directions
//!
//! The tempting invariant here is a count: on the sonic4 pair the debug shape
//! carries exactly one more equate than release (`ANI_PARTICLE`, from the debug-only
//! `games/sonic4/objects/test_particle.emp`), and that has held across several
//! revisions of the tree. IT IS THE WRONG INVARIANT, for two independent reasons
//! this file measures rather than asserts from memory:
//!
//!  * TWO POPULATIONS CAN DIFFER BY ONE MEMBER IN EACH DIRECTION AND TOTAL
//!    IDENTICALLY. A future RELEASE-ONLY equate is a real anomaly — an equate the
//!    debugger can read in the shipping ROM and not in the shape people debug with —
//!    and a count-difference check passes it straight through as long as some
//!    debug-only equate appears alongside it.
//!  * THE COUNT DIFFERENCE IS NOT EVEN CONSTANT ACROSS GAMES. Measured on this tree:
//!    sonic4 is 737/738 (release/debug) and demo is 555/555 — the demo pair's equate
//!    sets are EQUAL. A `debug == release + 1` gate is red on demo the day it is
//!    written; a `debug > release` gate is red on demo too. Containment is the
//!    property that is true of both games and stays true when a game grows a
//!    debug-only module.
//!
//! BOTH DIRECTIONS ARE COMPUTED. `release \ debug` is the fault set and fails the
//! gate; `debug \ release` is computed and REPORTED, so a reader sees the actual
//! asymmetry (and its membership) rather than inferring it from two totals. What is
//! deliberately NOT asserted is that `debug \ release` is non-empty: the demo pair is
//! symmetric today, and an assertion that fires on a correct tree trains people to
//! weaken it.

use sigil_harness::native::{self, GameProfile};
use sigil_harness::test_support::reference_tree_for_profile;
use sigil_link::ListingSymbol;
use std::collections::BTreeSet;
use std::path::Path;

/// The equate NAMES of one listing — the population containment is asserted over.
/// Names only: a value legitimately differs between shapes (`DEBUG` is 0 and 1), so
/// a value-sensitive comparison would report the shapes' whole point as a fault.
fn equate_names(listing: &[ListingSymbol]) -> BTreeSet<String> {
    listing.iter().filter(|s| s.is_equate).map(|s| s.name.clone()).collect()
}

/// THE CONTAINMENT ADJUDICATOR — pure, so a witness can drive it with sets it built
/// and watch it name them. Returns one fault line per release-only equate, and the
/// debug-only set for the caller to report.
///
/// `release ⊆ debug` is checked as a SET DIFFERENCE in the release-only direction.
/// The debug-only difference is returned rather than asserted on: see the file
/// header for why strictness is not the property.
fn containment_faults(
    game: &str,
    release: &BTreeSet<String>,
    debug: &BTreeSet<String>,
) -> (Vec<String>, BTreeSet<String>) {
    let release_only: BTreeSet<String> = release.difference(debug).cloned().collect();
    let debug_only: BTreeSet<String> = debug.difference(release).cloned().collect();
    let faults = if release_only.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "game `{game}`: {} equate(s) exist in the RELEASE listing and NOT in the \
             DEBUG listing: {:?}. Release must be a subset of debug — an equate the \
             shipping ROM publishes and the shape people debug with does not is a \
             symbol a crash report can name and no debug session can resolve. \
             (Totals: release {} / debug {}. A count check would have passed this \
             whenever a debug-only equate appeared alongside it, which is why the \
             assertion is containment.)",
            release_only.len(),
            release_only,
            release.len(),
            debug.len(),
        )]
    };
    (faults, debug_only)
}

/// The (release, debug) pairs. Only a GAME has both shapes; `config_a`/`config_b`/
/// `lean` are single off-canonical shapes with no counterpart, so they are outside
/// this property rather than silently folded into it.
fn shape_pairs() -> Vec<(&'static str, GameProfile, GameProfile)> {
    vec![
        ("sonic4", native::sonic4_profile(false), native::sonic4_profile(true)),
        ("demo", native::demo_profile(false), native::demo_profile(true)),
    ]
}

fn build_listing(aeon: &Path, label: &str, profile: &GameProfile) -> Result<Vec<ListingSymbol>, String> {
    native::build_rom_chained_with_listing(aeon, profile)
        .map(|b| b.listing)
        .map_err(|e| format!("shape `{label}`: build failed, so nothing was measured: {e}"))
}

/// GATE 1. Every shipped shape's listing carries every row of that shape's own
/// define env, at that shape's own value.
///
/// MUST FAIL: a define that reaches no listing row (the pre-parcel state, and the
/// state a lost wiring returns to); a define row whose listed value is not the value
/// the build compiled against; a shape whose listing carries no equates at all.
#[test]
fn every_shipped_shape_publishes_its_command_line_defines_in_its_listing() {
    let shapes = native::shipped_shapes();
    assert!(
        !shapes.is_empty(),
        "shipped_shapes() enumerated nothing — a walk over an empty set is green for \
         the one reason that proves nothing"
    );

    let mut aeon = None;
    for (_, profile) in &shapes {
        let Some(root) = reference_tree_for_profile(profile) else { return };
        aeon = Some(root);
    }
    let aeon = aeon.expect("shapes is non-empty, so the loop ran at least once");

    let mut faults: Vec<String> = Vec::new();
    let mut unmeasured: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for (label, profile) in &shapes {
        let listing = match build_listing(&aeon, label, profile) {
            Ok(l) => l,
            Err(e) => {
                unmeasured.push(e);
                continue;
            }
        };
        // THE EXPECTATION, derived from the profile + the game's map — the same call
        // the build's own lowering makes, never a transcribed name list.
        let expected = match native::shape_defines(profile, &aeon) {
            Ok(d) => d,
            Err(e) => {
                unmeasured.push(format!("shape `{label}`: define env unreadable: {e}"));
                continue;
            }
        };
        if expected.is_empty() {
            unmeasured.push(format!(
                "shape `{label}`: its define env is EMPTY, so charging its listing with \
                 carrying every row is a check over nothing"
            ));
            continue;
        }
        let equates: std::collections::HashMap<&str, u32> = listing
            .iter()
            .filter(|s| s.is_equate)
            .map(|s| (s.name.as_str(), s.value))
            .collect();
        if equates.is_empty() {
            unmeasured.push(format!(
                "shape `{label}`: its listing carries NO equate rows at all, so every \
                 per-define check below would report the same missing row for a reason \
                 that has nothing to do with defines"
            ));
            continue;
        }
        for (key, value) in &expected {
            checked += 1;
            let want = (*value as i64) as u32;
            match equates.get(key.as_str()) {
                None => faults.push(format!(
                    "shape `{label}`: define `{key}` (= {value}) reaches NO listing row. \
                     A tool asking this ROM for that value has to hardcode one, and the \
                     other game's value is different by construction"
                )),
                Some(got) if *got != want => faults.push(format!(
                    "shape `{label}`: define `{key}` is listed as ${got:08X} but the \
                     build compiled against {value} (${want:08X}). A listing row that \
                     disagrees with the build is worse than no row"
                )),
                Some(_) => {}
            }
        }
    }

    assert!(
        unmeasured.is_empty(),
        "{} of {} shipped shapes could not be measured, so this run says nothing about \
         them:\n  {}",
        unmeasured.len(),
        shapes.len(),
        unmeasured.join("\n  "),
    );
    eprintln!(
        "define rows: {checked} define(s) across {} shipped shapes reached their \
         listing at their own value",
        shapes.len()
    );
    assert!(
        faults.is_empty(),
        "{} define row(s) are missing or wrong in a shipped shape's listing:\n  {}",
        faults.len(),
        faults.join("\n  "),
    );
}

/// GATE 2. For every game, the RELEASE listing's equate names are a SUBSET of the
/// DEBUG listing's.
///
/// MUST FAIL: any equate present in a game's release listing and absent from its
/// debug listing — including the case where a debug-only equate appears alongside it
/// so that the two totals are equal, which is precisely what a count-difference
/// check cannot see. Also fails when either listing carries no equates (unmeasurable
/// is never green).
///
/// MUST NOT FAIL: a debug-only equate (that is the expected direction), and a game
/// whose two shapes have IDENTICAL equate sets (demo, today).
#[test]
fn every_games_release_equates_are_contained_in_its_debug_equates() {
    let pairs = shape_pairs();
    assert!(!pairs.is_empty(), "no (release, debug) pair to compare");

    let mut aeon = None;
    for (_, release, _) in &pairs {
        let Some(root) = reference_tree_for_profile(release) else { return };
        aeon = Some(root);
    }
    let aeon = aeon.expect("pairs is non-empty");

    let mut faults: Vec<String> = Vec::new();
    let mut unmeasured: Vec<String> = Vec::new();

    for (game, release_profile, debug_profile) in &pairs {
        let release = match build_listing(&aeon, release_profile.name, release_profile) {
            Ok(l) => equate_names(&l),
            Err(e) => {
                unmeasured.push(e);
                continue;
            }
        };
        let debug = match build_listing(&aeon, debug_profile.name, debug_profile) {
            Ok(l) => equate_names(&l),
            Err(e) => {
                unmeasured.push(e);
                continue;
            }
        };
        // LOUD ON UNMEASURABLE. An empty side makes containment trivially true in one
        // direction and trivially false in the other, and neither answer is about the
        // property.
        if release.is_empty() || debug.is_empty() {
            unmeasured.push(format!(
                "game `{game}`: equate populations are release {} / debug {} — a \
                 containment check needs both sides to exist",
                release.len(),
                debug.len(),
            ));
            continue;
        }
        let (mut game_faults, debug_only) = containment_faults(game, &release, &debug);
        eprintln!(
            "equate containment: game `{game}` release {} ⊆ debug {} ({} debug-only: {:?})",
            release.len(),
            debug.len(),
            debug_only.len(),
            debug_only,
        );
        faults.append(&mut game_faults);
    }

    assert!(
        unmeasured.is_empty(),
        "{} of {} game pairs could not be measured:\n  {}",
        unmeasured.len(),
        pairs.len(),
        unmeasured.join("\n  "),
    );
    assert!(faults.is_empty(), "{}", faults.join("\n  "));
}

/// RED-FIRST FOR THE ADJUDICATOR, on the case the rejected invariant cannot see:
/// two populations differing by one member IN EACH DIRECTION, totalling identically.
///
/// A count-difference check reads `3 == 3` and passes. Containment names `ONLY_REL`.
#[test]
fn a_release_only_equate_is_named_even_when_the_totals_match() {
    let release: BTreeSet<String> =
        ["SHARED_A", "SHARED_B", "ONLY_REL"].iter().map(|s| s.to_string()).collect();
    let debug: BTreeSet<String> =
        ["SHARED_A", "SHARED_B", "ONLY_DBG"].iter().map(|s| s.to_string()).collect();
    assert_eq!(release.len(), debug.len(), "the witness needs equal totals to be the witness");

    let (faults, debug_only) = containment_faults("witness", &release, &debug);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(faults[0].contains("ONLY_REL"), "{}", faults[0]);
    assert!(debug_only.contains("ONLY_DBG"), "{debug_only:?}");
}

/// The control the witness above needs: the SHIPPED direction — a debug-only equate,
/// with unequal totals — is green. Without this the gate above could be red for
/// every input, which is a delayed failure rather than a check.
#[test]
fn a_debug_only_equate_is_the_allowed_direction_and_is_not_a_fault() {
    let release: BTreeSet<String> = ["SHARED_A"].iter().map(|s| s.to_string()).collect();
    let debug: BTreeSet<String> =
        ["SHARED_A", "ANI_PARTICLE"].iter().map(|s| s.to_string()).collect();
    let (faults, debug_only) = containment_faults("witness", &release, &debug);
    assert!(faults.is_empty(), "{faults:?}");
    assert_eq!(debug_only.len(), 1, "{debug_only:?}");
}

/// The second control: EQUAL sets — the demo pair's shipped state — are green.
/// A gate that demanded strictness would be red here on a correct tree.
#[test]
fn identical_equate_sets_are_contained_and_not_a_fault() {
    let both: BTreeSet<String> =
        ["SHARED_A", "SHARED_B"].iter().map(|s| s.to_string()).collect();
    let (faults, debug_only) = containment_faults("witness", &both, &both);
    assert!(faults.is_empty(), "{faults:?}");
    assert!(debug_only.is_empty(), "{debug_only:?}");
}
