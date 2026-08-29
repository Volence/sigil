//! R7 — EVERY SHIPPED SHAPE SATISFIES ITS SECTIONS' DECLARED ALIGNMENT, and a repin
//! that breaks one is refused by name.
//!
//! WHAT THIS CLOSES. `native::packed_align_of` derives a section's packing quantum from
//! its frozen provisional base — the largest power of two in `{16,8,4,2}` dividing it.
//! Stated as a bound rather than as a procedure: **it only distinguishes residues mod
//! 16.** So a section's alignment has been a side effect of where it landed at the last
//! refreeze, not a declared property of the section, and the only gate covering the class
//! (`[sound.fold-vs-placement]`) covers TWO labels. `crates/sigil-harness/src/
//! section_align.rs` now declares the requirement per section with its source, and two
//! always-on halves check it:
//!
//!   * `native::validate_declared_alignment` — before the packing walk, against each
//!     pinned section's frozen provisional base (equivalently, for every requirement the
//!     inference can express, against the quantum it would infer; see the proof in that
//!     function's doc comment). Loud on a section with NO declaration.
//!   * `native::validate_resolved_alignment` — after `resolve_layout`, against the base
//!     each section ACTUALLY lands on. A different artifact from the frozen table, so it
//!     also covers the sections the walk places by some rule other than the inference:
//!     declared `[[anchor]]` islands, phase-bank hard orgs, the zero-byte-marker
//!     cap-at-2 path, and the label-less contiguity blobs that carry no pin at all.
//!
//! Both are wired inside `build_rom_chained_with_listing` — the entry `sigil build`
//! reaches — so every build of every shape runs them; this file is the witness that they
//! are green on the corpus AND that a violation is red.
//!
//! Reference tree: `AEON_DIR`, else the sibling aeon checkout. A missing tree skips
//! green outside strict mode and HARD-FAILS naming the absent path under
//! `SIGIL_STRICT_GATE=1`.

use sigil_harness::native;
use sigil_harness::section_align;
use sigil_harness::test_support::reference_tree_for_profile;

/// THE GATE: every shipped shape builds, which is exactly "every shipped shape's every
/// section satisfies its declared alignment" — both halves are always-on inside the
/// build entry, so a shape that violated one could not return `Ok`.
#[test]
fn every_shipped_shape_satisfies_its_declared_alignment() {
    let shapes = native::shipped_shapes();
    assert!(!shapes.is_empty(), "shipped_shapes() enumerated nothing — a gate over no shapes gates nothing");

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
         `[layout.undeclared-alignment]` or `[layout.alignment-violated]`):\n  {}",
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

/// RED-FIRST WITNESS, and it replays the real incident. Commit `2c49f538` moved the SFX
/// pin from `$5BAE8` to `$5BB10` and silently changed what the layout demanded of it;
/// aeon's `sfx_bank_blob.emp` requires `Sfx_33`'s base ≡ 0 (mod 8) or every folded sound
/// pointer lands short of its blob, silently. Doctor that ONE frozen row by +4 and the
/// build must be REFUSED naming the section, the requirement, the source that requires
/// it, and the residue — so a green above is a measurement rather than an absence of
/// detection.
///
/// +4 and not +2: the requirement is 8, so the doctored value must break mod 8 while
/// staying even (an odd base would be caught by the word rule instead, which would prove
/// a different check).
#[test]
fn a_repin_that_breaks_a_declared_alignment_is_refused_by_name() {
    let mut profile = native::sonic4_profile(false);
    let Some(aeon) = reference_tree_for_profile(&profile) else { return };

    const PROBE: &str = "Sfx_33";
    let declared = section_align::required_for(PROBE)
        .unwrap_or_else(|| panic!("`{PROBE}` must be declared for this witness to mean anything"));
    assert_eq!(declared.required, 8, "this witness is written against a mod-8 requirement");

    let native::SizeSource::Frozen(t) = &mut profile.size_source else {
        panic!("the sonic4 profile must be a frozen-table target for this witness");
    };
    let before = *t.get(PROBE).unwrap_or_else(|| panic!("`{PROBE}` not in the frozen table"));
    assert!(before.is_multiple_of(8), "control: the shipped pin {before:#x} already satisfies 8");
    t.insert(PROBE.to_string(), before + 4);

    let e = match native::build_rom_chained_with_listing(&aeon, &profile) {
        Err(e) => e,
        Ok(_) => panic!("a pin 4 bytes off a mod-8 requirement must be REFUSED, not built"),
    };
    assert!(e.contains("[layout.undeclared-alignment]"), "wrong refusal: {e}");
    assert!(e.contains(PROBE), "the refusal must name the section: {e}");
    assert!(e.contains("declares alignment 8"), "the refusal must name the requirement: {e}");
    assert!(e.contains("sfx_bank_blob.emp"), "the refusal must name the source: {e}");
    assert!(e.contains("base % 8 = 4"), "the refusal must name the residue: {e}");
}

/// THE CAP'S BLIND SPOT, with the numbers. Three sections require more than 16 — the two
/// Z80 bank heads require `$8000` (one `SetBank` window, `bankid() = (lma & $7F8000) >>
/// 15`) and `ObjCodeBase` requires `$10000` (aeon's R1 ruling) — and `packed_align_of`
/// reports **16** for every one of their anchor addresses. Those requirements are
/// INEXPRESSIBLE in the inference, which is why the pre-walk half reads the provisional
/// base directly rather than the inferred quantum.
#[test]
fn the_requirements_above_the_cap_exceed_what_the_inference_can_express() {
    for (label, at, want) in [
        ("Dac_Temp_Blip", 0x90000u32, 0x8000u32),
        ("SoundTablesZ80_Head", 0xA0000, 0x8000),
        ("ObjCodeBase", 0x10000, 0x10000),
    ] {
        let decl = section_align::required_for(label)
            .unwrap_or_else(|| panic!("`{label}` must be declared"));
        assert_eq!(decl.required, want, "`{label}`'s declared requirement");
        assert_eq!(
            native::packed_align_of(at),
            16,
            "`{label}` at {at:#x} infers 16 — the mod-16 cap cannot see its real requirement"
        );
        assert!(at.is_multiple_of(decl.required), "`{label}`'s anchor must satisfy its requirement");
    }
}
