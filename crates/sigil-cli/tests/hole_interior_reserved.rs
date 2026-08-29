//! HOLE-INTERIOR-RESERVED over the SHIPPED shapes — the `[[hole]]` half of the
//! placement contract, driven through `native::validate_placement` (the post-resolve
//! gate `build_rom_chained_with_listing` runs) against each shape's real `map.toml` and
//! the real resolve the ROM comes from.
//!
//! A `[[hole]]` is a reserved empty span: it opens at its `after` label, runs to `at`,
//! and the module `filled_by` names is the one thing allowed inside. The unit fixtures
//! in `native::placement_validation_tests` prove the predicate's shape; this file proves
//! it over the layouts that actually ship, in both directions:
//!
//!   * GREEN — every live hole in every shipped shape holds nothing but its filler;
//!   * RED — moving one hole's declared right edge past the post-hole data (in memory,
//!     never in the aeon tree) makes the build path refuse, naming the section that
//!     lands in the reserved span and the bytes of it that do.
//!
//! POPULATION. Four of the seven shipped shapes gate their hole out with
//! `when = "sound_off"`, and over those the predicate correctly returns nothing. So a
//! green run here is only coverage if some shape declares a LIVE hole, and
//! [`some_shipped_shape_declares_a_live_hole`] is the guard that says so out loud —
//! without it this file would go on reporting green the day the last live hole is
//! gated away.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test hole_interior_reserved
//! ```
use sigil_harness::map_placement::PlacementMap;
use sigil_harness::native;
use sigil_harness::test_support::reference_tree_for_profile;

// The frozen resolve touches the shared engine/sound/generated dir.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `profile`'s real placement map, read from the aeon tree the build reads it from.
fn map_for(aeon: &std::path::Path, profile: &native::GameProfile) -> PlacementMap {
    let path = profile.map_path(aeon);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    sigil_harness::map_placement::load_placement_map(&src)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every shipped shape that declares at least one hole IN ITS OWN `when` gate, paired
/// with the map that declares it. Read off the maps alone — no build — so the
/// population is established before any expensive resolve is paid for.
fn shapes_with_a_live_hole(
    aeon: &std::path::Path,
) -> Vec<(&'static str, native::GameProfile, PlacementMap)> {
    native::shipped_shapes()
        .into_iter()
        .filter_map(|(name, profile)| {
            let m = map_for(aeon, &profile);
            let live = m.holes_for(profile.sound_on).count();
            (live > 0).then_some((name, profile, m))
        })
        .collect()
}

/// THE POPULATION GUARD. `Ok(vec![])` over a shape whose `when` gates every hole out is
/// a correct empty answer and not coverage, so this file's green is worth nothing until
/// some shipped shape is shown to declare a live hole. Names them, so a run that loses
/// one is visible in the log rather than silently cheaper.
#[test]
fn some_shipped_shape_declares_a_live_hole() {
    let Some(aeon) = reference_tree_for_profile(&native::demo_profile(false)) else {
        return;
    };
    let live = shapes_with_a_live_hole(&aeon);
    let names: Vec<&str> = live.iter().map(|(n, _, _)| *n).collect();
    assert!(
        !live.is_empty(),
        "no shipped shape declares a live `[[hole]]` — every assertion in this file is \
         vacuous, and the hole contract has no subject to be checked against"
    );
    eprintln!("live-hole shapes: {names:?}");
}

/// GREEN: over every shipped shape with a live hole, the real map and the real resolve
/// agree — the reserved interior holds nothing but the module `filled_by` names.
#[test]
fn every_live_hole_holds_only_its_filler() {
    let Some(aeon) = reference_tree_for_profile(&native::demo_profile(false)) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut checked = 0usize;
    for (name, profile, m) in shapes_with_a_live_hole(&aeon) {
        let Some(aeon) = reference_tree_for_profile(&profile) else {
            continue;
        };
        let resolved = native::resolve_frozen_layout(&aeon, &profile)
            .unwrap_or_else(|e| panic!("{name}: resolve: {e}"));
        native::validate_placement(&resolved, &m, profile.sound_on, &profile.registry)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        checked += 1;
    }
    assert!(checked > 0, "no shape was measured — see the population guard");
}

/// RED-FIRST, through the same call the ROM build makes: a hole whose declared right
/// edge reaches past the post-hole data must be refused, naming the intruding section
/// and the bytes of it inside the reserved span.
///
/// EVERY NUMBER IS DERIVED FROM THE RESOLVE. The doctored right edge is the END of the
/// first byte-emitting section at or after the real `at` that the hole does not permit;
/// the expected occupied span is that section's own bounds. Nothing is transcribed, and
/// the aeon tree is never written to — only the in-memory `PlacementMap` is doctored.
#[test]
fn a_right_edge_past_the_post_hole_data_is_refused() {
    let Some(aeon) = reference_tree_for_profile(&native::demo_profile(false)) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut proven = 0usize;
    for (name, profile, m) in shapes_with_a_live_hole(&aeon) {
        let Some(aeon) = reference_tree_for_profile(&profile) else {
            continue;
        };
        let resolved = native::resolve_frozen_layout(&aeon, &profile)
            .unwrap_or_else(|e| panic!("{name}: resolve: {e}"));

        // Index into `m.holes` (not into the gated view), so the doctored right edge
        // lands on the hole this iteration is about.
        let live: Vec<usize> = m
            .holes
            .iter()
            .enumerate()
            .filter(|(_, h)| m.holes_for(profile.sound_on).any(|g| std::ptr::eq(g, *h)))
            .map(|(i, _)| i)
            .collect();
        for i in live {
            let hole = &m.holes[i];
            // The sections this hole permits inside itself, from the shape's registry —
            // the same derivation the predicate makes.
            let permitted: Vec<&str> = profile
                .registry
                .iter()
                .filter(|ms| ms.module_id == hole.filled_by)
                .map(|ms| ms.section)
                .collect();
            assert!(
                !permitted.is_empty(),
                "{name}: hole after `{}` is filled_by `{}`, which names no module in \
                 this shape's registry",
                hole.after,
                hole.filled_by
            );

            // The first emitter at or after the declared right edge that the hole does
            // NOT permit: extending `at` to its end puts exactly that section inside.
            let mut post: Vec<(u32, u32, &str)> = resolved
                .iter()
                .filter(|s| native::is_rom_section(s))
                .filter(|s| !permitted.contains(&s.name.as_str()))
                .map(|s| (s.lma, s.image_bytes().len() as u32, s.name.as_str()))
                .filter(|(lma, len, _)| *len > 0 && *lma >= hole.at)
                .collect();
            post.sort_by_key(|(lma, _, _)| *lma);
            let (lma, len, section) = *post.first().unwrap_or_else(|| {
                panic!(
                    "{name}: nothing emits at or after the hole's declared `at` \
                     ({:#X}), so the red direction cannot be constructed",
                    hole.at
                )
            });
            let doctored_at = lma + len;

            let mut doctored = m.clone();
            doctored.holes[i].at = doctored_at;
            let e = native::validate_placement(
                &resolved,
                &doctored,
                profile.sound_on,
                &profile.registry,
            )
            .expect_err(&format!(
                "{name}: `{section}` at [{lma:#X},{doctored_at:#X}) is inside the \
                 doctored hole and the build path accepted it"
            ));
            assert!(e.contains("map.hole-interior-occupied"), "{name}: {e}");
            assert!(
                e.contains(&format!("`{section}`")),
                "{name}: must name the intruding section `{section}`: {e}"
            );
            assert!(
                e.contains(&format!("[{lma:#X},{doctored_at:#X})")),
                "{name}: must name the occupied span: {e}"
            );
            assert_eq!(e.lines().count(), 1, "{name}: one intruder, one fault: {e}");

            // CONTROL, the other direction: the undoctored map over the identical
            // resolve passes, so the red is the moved right edge and nothing else.
            native::validate_placement(&resolved, &m, profile.sound_on, &profile.registry)
                .unwrap_or_else(|e| panic!("{name}: control: {e}"));
            proven += 1;
        }
    }
    assert!(proven > 0, "no hole was driven red — see the population guard");
}
