//! R7 — EVERY SHIPPED SHAPE SATISFIES ITS SECTIONS' DECLARED ALIGNMENT, the declaration
//! is the packing walk's ONLY alignment input, and a section it cannot place is refused
//! by name.
//!
//! `crates/sigil-harness/src/section_align.rs` declares the alignment each ROM section
//! REQUIRES with its source. `native::packed_chained_base` rounds every chained section's
//! base to that row and nothing else — no frozen provisional base enters the arithmetic —
//! and `seam2::sound_layout` predicts the two sound blobs' bases through the same function
//! with the same head labels. Two always-on halves gate it:
//!
//!   * `native::validate_declared_alignment` — before the packing walk: every ROM section
//!     with a head label has a row, pinned or not. Loud on a section with NO declaration.
//!   * `native::validate_resolved_alignment` — after `resolve_layout`, against the base
//!     each section ACTUALLY lands on: the independent instrument for the sections the
//!     walk places by a rule other than the declaration (declared `[[anchor]]` islands,
//!     phase-bank hard orgs, label-less contiguity blobs).
//!
//! Both are wired inside `build_rom_chained_with_listing` — the entry `sigil build`
//! reaches — so every build of every shape runs them, and `validate_sound_fold` (the
//! seam2-vs-walk agreement witness, always-on on every sound-on shape) runs beside them.
//! This file is the witness that they are green on the corpus AND that the frozen
//! tables are not an input.
//!
//! Reference tree: `AEON_DIR`, else the sibling aeon checkout. A missing tree skips
//! green outside strict mode and HARD-FAILS naming the absent path under
//! `SIGIL_STRICT_GATE=1`.

use sigil_harness::native;
use sigil_harness::section_align;
use sigil_harness::test_support::reference_tree_for_profile;

/// THE GATE: every shipped shape builds, which is exactly "every shipped shape's every
/// section satisfies its declared alignment, and seam2's prediction agrees with the
/// walk's placement" — all three checks are always-on inside the build entry, so a shape
/// that violated one could not return `Ok`.
#[test]
fn every_shipped_shape_satisfies_its_declared_alignment() {
    let shapes = native::shipped_shapes();
    assert!(!shapes.is_empty(), "shipped_shapes() enumerated nothing, a gate over no shapes gates nothing");

    let mut aeon = None;
    for (_, profile) in &shapes {
        let Some(root) = reference_tree_for_profile(profile) else { return };
        aeon = Some(root);
    }
    let aeon = aeon.expect("shapes is non-empty, so the loop ran at least once");

    // Measure every shape before judging any, so a run names EVERY violator.
    let faults: Vec<String> = shapes
        .iter()
        .filter_map(|(label, profile)| {
            native::build_rom_chained_with_listing(&aeon, profile)
                .err()
                .map(|e| format!("shape `{label}`: {e}"))
        })
        .collect();
    assert!(
        faults.is_empty(),
        "{} of {} shipped shapes fail a placement check (an alignment fault reads \
         `[layout.undeclared-alignment]` or `[layout.alignment-violated]`; a prediction \
         fault reads `[sound.fold-vs-placement]`):\n  {}",
        faults.len(),
        shapes.len(),
        faults.join("\n  ")
    );
    eprintln!(
        "declared alignment: {} shipped shapes green over {} declared sections at {}",
        shapes.len(),
        section_align::DECLARED.len(),
        aeon.display()
    );
}

/// The resolved address of `label` in a built shape's listing.
fn listed_addr(listing: &[sigil_link::ListingSymbol], label: &str) -> u32 {
    listing
        .iter()
        .find(|s| s.name == label && !s.is_equate)
        .unwrap_or_else(|| panic!("`{label}` is not in the listing"))
        .value
}

/// THE FROZEN TABLE IS NOT AN ALIGNMENT INPUT — the witness that the flip is the
/// mechanism and not a coincidence of aligned cursors. Commit `2c49f538` moved the SFX
/// pin from `$5BAE8` to `$5BB10` and, because the quantum was read off the pin's residue,
/// silently doubled what the layout demanded of it. Doctor that ONE frozen row by +4 —
/// enough to break mod 8 while staying even — and the build must go through UNCHANGED:
/// `Sfx_33` lands at the same base as the undoctored build, and that base satisfies the
/// declaration (8). A walk that still read the pin would either refuse (the pre-flip
/// gate) or move the section; either fails here.
///
/// This test's own falsifier is the pre-flip packer: run against it, the doctored build
/// is refused with `[layout.undeclared-alignment] … base % 8 = 4` and the `Ok` arm below
/// is never reached.
#[test]
fn a_doctored_pin_residue_does_not_move_a_packed_section() {
    let profile = native::sonic4_profile(false);
    let Some(aeon) = reference_tree_for_profile(&profile) else { return };

    const PROBE: &str = "Sfx_33";
    let declared = section_align::required_for(PROBE)
        .unwrap_or_else(|| panic!("`{PROBE}` must be declared for this witness to mean anything"));
    assert_eq!(declared.required, 8, "this witness is written against a mod-8 requirement");

    let control = native::build_rom_chained_with_listing(&aeon, &profile)
        .unwrap_or_else(|e| panic!("control build: {e}"));
    let control_base = listed_addr(&control.listing, PROBE);
    assert!(control_base.is_multiple_of(8), "control: `{PROBE}` at {control_base:#x} satisfies 8");

    let mut doctored = native::sonic4_profile(false);
    let t = &mut doctored.frozen_sizes;
    let before = *t.get(PROBE).unwrap_or_else(|| panic!("`{PROBE}` not in the frozen table"));
    assert!(before.is_multiple_of(8), "control: the shipped pin {before:#x} already satisfies 8");
    t.insert(PROBE.to_string(), before + 4);

    let built = match native::build_rom_chained_with_listing(&aeon, &doctored) {
        Ok(b) => b,
        Err(e) => panic!("a pin's residue is not an alignment input; the build must go through, got: {e}"),
    };
    let doctored_base = listed_addr(&built.listing, PROBE);
    assert_eq!(
        doctored_base, control_base,
        "`{PROBE}` moved with the pin's residue, the frozen table is still an alignment input"
    );
    assert_eq!(built.rom, control.rom, "the doctored pin must not move a byte");
}

/// The rows above 16 the walk HOLDS: a frozen row equal to a declared `[[anchor]]` (or a
/// phase bank) places them absolute, and only `validate_resolved_alignment` measures
/// them against the declaration.
const HELD_ABOVE_16: &[(&str, u32)] =
    &[("Dac_Temp_Blip", 0x8000), ("SoundTablesZ80_Head", 0x8000), ("ObjCodeBase", 0x10000)];

/// The rows above 16 the walk ROUNDS: no frozen row names them, so the running cursor
/// is rounded up to the declaration by `native::packed_chained_base`, and an aeon
/// `[[anchor]]` at the landing address is a lint, not a placement input.
const ROUNDED_ABOVE_16: &[(&str, u32)] = &[("SongBank2_Head", 0x8000)];

/// THE REQUIREMENTS ABOVE 16, as declared, and the rule that places each. The Z80 bank
/// heads require `$8000` (one `SetBank` window, `bankid() = (lma & $7F8000) >> 15`) and
/// `ObjCodeBase` requires `$10000` (aeon's R1 ruling). Every row above 16 is either HELD
/// or ROUNDED, and the two are told apart by the one input that decides it: whether a
/// committed frozen table names the label. A HELD row with no frozen row anywhere would
/// pack by rounding and drift off its anchor; a ROUNDED row that gained a frozen row
/// would become a frozen-held island, so every later move would need a paired
/// frozen-row and anchor edit. The partition is read off `DECLARED` itself, so a new
/// row above 16 fails here until it is classified.
#[test]
fn the_requirements_above_16_are_declared_held_or_rounded() {
    for (label, want) in HELD_ABOVE_16.iter().chain(ROUNDED_ABOVE_16) {
        let decl = section_align::required_for(label)
            .unwrap_or_else(|| panic!("`{label}` must be declared"));
        assert_eq!(decl.required, *want, "`{label}`'s declared requirement");
    }

    let classified: Vec<&str> =
        HELD_ABOVE_16.iter().chain(ROUNDED_ABOVE_16).map(|(l, _)| *l).collect();
    let unclassified: Vec<&str> = section_align::DECLARED
        .iter()
        .filter(|d| d.required > 16 && !classified.contains(&d.label))
        .map(|d| d.label)
        .collect();
    assert!(unclassified.is_empty(), "rows above 16 classified neither HELD nor ROUNDED: {unclassified:?}");

    let shapes = native::shipped_shapes();
    assert!(!shapes.is_empty(), "shipped_shapes() enumerated nothing, the frozen-row check would read no table");
    let frozen_in = |label: &str| -> Vec<(String, u32)> {
        shapes
            .iter()
            .filter_map(|(shape, p)| p.frozen_sizes.get(label).map(|a| (shape.to_string(), *a)))
            .collect()
    };
    for (label, want) in HELD_ABOVE_16 {
        let rows = frozen_in(label);
        assert!(
            !rows.is_empty(),
            "HELD `{label}` has no frozen row in any shipped shape, so the walk would round it \
             instead of holding it at its anchor"
        );
        for (shape, addr) in rows {
            assert!(
                addr.is_multiple_of(*want),
                "HELD `{label}`'s frozen row in `{shape}` is {addr:#x}, not a multiple of {want:#x}"
            );
        }
    }
    for (label, _) in ROUNDED_ABOVE_16 {
        let rows = frozen_in(label);
        assert!(
            rows.is_empty(),
            "ROUNDED `{label}` has a frozen row {rows:?}; the walk would hold it there as an \
             island rather than round it"
        );
    }
}
