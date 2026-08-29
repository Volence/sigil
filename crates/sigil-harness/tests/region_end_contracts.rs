//! R6 — every `[[region]]`'s END CONTRACT, checked against the live layout.
//!
//! **What R6 is about.** `repin.toml` pins a region as `end − start`, both resolved from
//! the symbol listing. When `end` is a label the region does not own — overwhelmingly the
//! NEXT section's head — the pin measures the gap to a neighbour, not this region's size,
//! and the placer's pad between them enters the pin silently. That is not hypothetical:
//! a refreeze put `OJZ_Sec0_Blocks` two bytes past the act descriptor and the successor's
//! pad entered `ACT_DESCRIPTOR` (0x27C pinned against 0x27A of real content). Under fresh
//! placement the neighbour is no longer where it was, so every such pin is a mis-measure
//! waiting rather than a stable convention.
//!
//! **What is declared now.** Each region says what its `end` value is a statement ABOUT:
//! `end = "section:<name>"` (the region's own extent), a literal `len` (a fixed-size
//! blob), or `end_measures = "allotment"` (the end is deliberately the next placement,
//! and the width is NOT a size). The DEFAULT is the strict reading, so a region that says
//! nothing is asserting that its window holds its own bytes and nothing past them.
//!
//! **No byte count is declared, deliberately.** The pad is a property of where the
//! successor landed, not of this region; writing today's number into the manifest would
//! enshrine an accident as a requirement — the mistake the R7 alignment parcel measured
//! its way out of. What is declared is the KIND of contract, which is stable across
//! refreezes.
//!
//! **Which instrument each half uses** (a gate that asks the same resolver the pin file
//! asked cannot notice that resolver being wrong): the window comes from the SYMBOL
//! listing (`native::sigil_native_symbol_listing`), the content extent from the SECTION
//! TABLE of the resolved layout the ROM is emitted from (`native::section_extents` →
//! `resolve_canonical_sections`). Two derivations, so a symbol address that disagrees
//! with the section geometry is visible here. Neither half can catch a wrong resolve —
//! both come from one link of one tree — and this file does not claim otherwise.
//!
//! REFERENCE-DEPENDENT and SOURCE-ONLY (`AEON_DIR`; `SIGIL_STRICT_GATE=1` makes a missing
//! reference fatal). No built ROM, no assembler listing.

use sigil_harness::native;
use sigil_harness::repin::{load_manifest, resolve, Listing};
use sigil_harness::test_support::reference_tree_for_profile;

const MANIFEST: &str = include_str!("../repin.toml");

/// Both shapes' listings, each carrying the phase-bank LMA map and the section table.
fn live_listings() -> Option<(Listing, Listing)> {
    let aeon = reference_tree_for_profile(&native::sonic4_profile(false))?;
    let (pm, pe) = native::sigil_native_symbol_listing(&aeon, false)
        .unwrap_or_else(|e| panic!("plain resolve: {e}"));
    let (dm, de) = native::sigil_native_symbol_listing(&aeon, true)
        .unwrap_or_else(|e| panic!("debug resolve: {e}"));
    let pl = native::phase_bank_lmas(&aeon, false).unwrap_or_else(|e| panic!("plain phase lma: {e}"));
    let dl = native::phase_bank_lmas(&aeon, true).unwrap_or_else(|e| panic!("debug phase lma: {e}"));
    let ps = native::section_extents(&aeon, false).unwrap_or_else(|e| panic!("plain sections: {e}"));
    let ds = native::section_extents(&aeon, true).unwrap_or_else(|e| panic!("debug sections: {e}"));
    assert!(!ps.is_empty() && !ds.is_empty(), "a shape with no section table cannot judge any end");
    let po = native::section_label_owners(&aeon, false).unwrap_or_else(|e| panic!("plain owners: {e}"));
    let do_ = native::section_label_owners(&aeon, true).unwrap_or_else(|e| panic!("debug owners: {e}"));
    assert!(!po.is_empty() && !do_.is_empty(), "an empty ownership map silently skips the flush check");
    Some((
        Listing::from_symbols(pm, pe, "plain".into())
            .with_phase_lma(pl)
            .with_sections(ps)
            .with_label_owners(po),
        Listing::from_symbols(dm, de, "debug".into())
            .with_phase_lma(dl)
            .with_sections(ds)
            .with_label_owners(do_),
    ))
}

/// The names of the regions that carry `end_measures = "allotment"`, read from the
/// manifest TEXT rather than from a count kept in this file — so the test cannot fall out
/// of step with the manifest, and adding or removing a declaration needs no edit here.
fn declared_allotments() -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<&str> = None;
    for line in MANIFEST.lines() {
        let t = line.trim();
        if t == "[[region]]" {
            current = None;
        } else if let Some(rest) = t.strip_prefix("name = \"") {
            if current.is_none() {
                current = rest.strip_suffix('"');
            }
        } else if t == "end_measures = \"allotment\"" {
            out.push(current.expect("`end_measures` inside a region with a name").to_string());
        }
    }
    out
}

/// Every region's declared end contract holds against the live layout of BOTH shapes,
/// and no declaration is stale.
///
/// Two directions, and they are not the same assertion:
/// - a region with NO declaration whose window sweeps a neighbour's pad makes `resolve`
///   fail (the strict default) — this test asserts the whole manifest resolves, so every
///   such region is either converted or declared;
/// - a region that DOES declare an allotment it no longer has produces a "zero-width"
///   advisory, and this test fails on it. That is the ratchet: the population of
///   neighbour-dependent ends can shrink without an edit here, but it cannot be left
///   overstated. `repin` itself only warns, so the tool can always regenerate.
#[test]
fn every_region_end_contract_holds_against_the_live_layout() {
    let Some((plain, debug)) = live_listings() else {
        return;
    };
    let manifest = load_manifest(MANIFEST).expect("repin.toml must load");
    let resolved = resolve(&manifest, &plain, &debug)
        .unwrap_or_else(|e| panic!("a region's end contract is violated:\n{e}"));

    // A declaration is per REGION, and a region's shapes disagree: 15 of them carry pad
    // in one shape and sit flush in the other (`boot` pads 8 in plain and 0 in debug).
    // So "stale" is a per-region verdict over ALL shapes — a zero-width note in one shape
    // is information, not a defect. Judging it per shape would demand a conversion that
    // moves the other shape's pin.
    let declared = declared_allotments();
    assert!(!declared.is_empty(), "the manifest declares no allotments — did the parse break?");
    let carries_pad = |name: &str| {
        resolved
            .warnings
            .iter()
            .any(|w| w.contains(&format!("region `{name}` (")) && w.contains("declared allotment"))
    };
    let stale: Vec<&String> = declared.iter().filter(|n| !carries_pad(n)).collect();
    assert!(
        stale.is_empty(),
        "{} region(s) declare an allotment they no longer have in ANY shape — convert each \
         to `end = \"section:<name>\"` and drop the declaration (the pin does not move): {}",
        stale.len(),
        stale.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
    );
}

/// RED-FIRST, PERMANENTLY, AGAINST THE REAL CORPUS: for every region that declares
/// `end_measures = "allotment"`, deleting that one line must make the live manifest
/// REFUSE to resolve — naming the region, the section its content ends in, and both
/// remedies. A gate nobody has seen fail proves nothing; this one fails ~50 times on
/// every run, on the actual layout, and asserts the refusal is legible.
///
/// Nothing here asserts a byte count. The number is an accident of where the successor
/// landed; the assertion is that the dependency is NAMED.
#[test]
fn deleting_an_allotment_declaration_refuses_the_live_manifest() {
    let Some((plain, debug)) = live_listings() else {
        return;
    };
    let declared = declared_allotments();
    assert!(!declared.is_empty(), "the manifest declares no allotments — did the parse break?");
    for name in &declared {
        let doctored = strip_one_declaration(name);
        assert_ne!(doctored, MANIFEST, "region `{name}`: nothing was stripped");
        let manifest = load_manifest(&doctored)
            .unwrap_or_else(|e| panic!("region `{name}`: doctored manifest must still parse: {e}"));
        let err = match resolve(&manifest, &plain, &debug) {
            Err(e) => e,
            Ok(_) => panic!(
                "region `{name}`: the manifest resolved with its allotment declaration \
                 REMOVED — the strict default is not reaching this region, or its pad has \
                 gone to zero (in which case convert it to `end = \"section:<name>\"`)"
            ),
        };
        assert!(err.contains(&format!("region `{name}` (")), "the refusal names the region: {err}");
        // Either refusal is correct and which one fires is shape-dependent: a shape that
        // pads reports the pad; a shape that is flush reports that the end label belongs
        // to another section. `resolve` stops at the first, so accept both.
        assert!(
            err.contains("placer pad past section `") || err.contains("is defined in section `"),
            "the refusal names the section the content ends in: {err}"
        );
        assert!(
            err.contains("section:") && err.contains("end_measures = \"allotment\""),
            "the refusal names both remedies: {err}"
        );
    }
}

/// Remove the single `end_measures = "allotment"` line belonging to `region`.
fn strip_one_declaration(region: &str) -> String {
    let mut out = Vec::new();
    let mut current: Option<&str> = None;
    let mut done = false;
    for line in MANIFEST.lines() {
        let t = line.trim();
        if t == "[[region]]" {
            current = None;
        } else if let Some(rest) = t.strip_prefix("name = \"") {
            if current.is_none() {
                current = rest.strip_suffix('"');
            }
        }
        if !done && t == "end_measures = \"allotment\"" && current == Some(region) {
            done = true;
            continue;
        }
        out.push(line);
    }
    out.join("\n")
}
