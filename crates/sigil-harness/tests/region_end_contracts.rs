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
//! **What is declared now.** A region's `end` is `end = "section:<name>"` (the region's
//! own extent), a literal `len` (a fixed-size blob), or a bare label, which is read
//! STRICTLY: the window holds this region's bytes and nothing past them, and the label is
//! defined in the section the region's bytes end in. There is no other reading.
//!
//! **The `allotment` contract is retired (walk-measure-in-place, 2026-11-19).** R6 also let
//! a region declare `end_measures = "allotment"`: the end is the next placement, and the
//! width is not a size. 50 regions started there and were converted to `section:` ends
//! over later parcels; the last, `player_climb`, lost its pad when the old walk began
//! measuring every ROM section in place. The pad every allotment accounted for was
//! scratch-measurement padding, a port-history artifact, and with no user left the kind
//! is gone from the parser: a manifest that still writes `end_measures` is refused as an
//! unknown field. A future game that needs a gap-to-neighbour pin brings the kind back
//! with a real user.
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

use std::collections::HashMap;

use sigil_harness::native;
use sigil_harness::repin::{load_manifest, resolve, Listing, SectionExtent, SECTION_PREFIX};
use sigil_harness::test_support::reference_tree_for_profile;

const MANIFEST: &str = include_str!("../repin.toml");

/// The live inputs: both shapes' listings (each carrying the phase-bank LMA map, the
/// section table and the label-ownership map), plus the plain shape's raw symbol table,
/// section table and ownership map for the negative control to choose its doctoring from.
struct Live {
    plain: Listing,
    debug: Listing,
    plain_syms: HashMap<String, u32>,
    debug_syms: HashMap<String, u32>,
    plain_sections: Vec<SectionExtent>,
    plain_owners: HashMap<String, String>,
}

fn live() -> Option<Live> {
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
    Some(Live {
        plain: Listing::from_symbols(pm.clone(), pe, "plain".into())
            .with_phase_lma(pl)
            .with_sections(ps.clone())
            .with_label_owners(po.clone()),
        debug: Listing::from_symbols(dm.clone(), de, "debug".into())
            .with_phase_lma(dl)
            .with_sections(ds)
            .with_label_owners(do_),
        plain_syms: pm,
        debug_syms: dm,
        plain_sections: ps,
        plain_owners: po,
    })
}

/// The number of `[[region]]` tables in the manifest TEXT, counted by a line scan that
/// shares nothing with the TOML parser. It is the floor both tests hold the parser and
/// the resolver to, so it is derived from `repin.toml` itself, never typed here.
fn declared_region_count() -> usize {
    MANIFEST.lines().filter(|l| l.trim() == "[[region]]").count()
}

/// Every region's end contract holds against the live layout of BOTH shapes, and every
/// declared region was judged.
///
/// The "did the parse break?" guard is the population that exists: the `[[region]]`
/// tables the manifest text declares. The parser must return all of them and the
/// resolver must pin all of them, so a parse that silently drops a region, or a resolve
/// that skips one, is red here rather than a smaller manifest that still passes.
#[test]
fn every_region_end_contract_holds_against_the_live_layout() {
    let Some(live) = live() else {
        return;
    };
    let declared = declared_region_count();
    assert!(declared > 0, "repin.toml declares no `[[region]]`, did the manifest move?");
    let manifest = load_manifest(MANIFEST).expect("repin.toml must load");
    assert_eq!(
        manifest.regions.len(),
        declared,
        "the parser returned {} region(s) but the manifest text declares {declared}: the parse broke",
        manifest.regions.len()
    );
    let resolved = resolve(&manifest, &live.plain, &live.debug)
        .unwrap_or_else(|e| panic!("a region's end contract is violated:\n{e}"));
    assert_eq!(
        resolved.regions.len(),
        declared,
        "the resolver pinned {} region(s) of the {declared} the manifest declares",
        resolved.regions.len()
    );
}

/// One doctoring target: a region whose `end = "section:<own>"` is re-spelled as the head
/// label of the section placed right after `<own>`, the exact mis-measure R6 exists for.
struct Target {
    region: String,
    own: String,
    successor: String,
    label: String,
}

/// Every both-shape `section:` region whose plain successor section has a head label: a
/// label the ownership map assigns to the successor, at the successor's LMA, that the
/// debug listing defines too (so the doctored manifest fails on the contract, not on a
/// missing symbol). Derived from the manifest and the live layout; nothing is listed here.
fn doctoring_targets(live: &Live) -> (Vec<Target>, usize) {
    let manifest = load_manifest(MANIFEST).expect("repin.toml must load");
    let mut sections = live.plain_sections.clone();
    sections.sort_by_key(|s| (s.lma, s.end));
    let mut targets = Vec::new();
    let mut candidates = 0;
    for r in &manifest.regions {
        if r.debug_only || r.plain_only || r.debug_end.is_some() {
            continue;
        }
        let Some(own) = r.end.as_deref().and_then(|e| e.strip_prefix(SECTION_PREFIX)) else {
            continue;
        };
        candidates += 1;
        let Some(cur) = sections.iter().find(|s| s.name == own) else {
            continue;
        };
        let Some(next) = sections.iter().find(|s| s.lma >= cur.end && s.name != own) else {
            continue;
        };
        let mut heads: Vec<&String> = live
            .plain_owners
            .iter()
            .filter(|(label, owner)| {
                owner.as_str() == next.name
                    && live.plain_syms.get(label.as_str()) == Some(&next.lma)
                    && live.debug_syms.contains_key(label.as_str())
                    && !label.contains('.')
            })
            .map(|(label, _)| label)
            .collect();
        heads.sort();
        if let Some(label) = heads.first() {
            targets.push(Target {
                region: r.name.clone(),
                own: own.to_string(),
                successor: next.name.clone(),
                label: (*label).clone(),
            });
        }
    }
    (targets, candidates)
}

/// Re-spell ONE region's `end = "section:<own>"` line as `end = "<label>"`.
fn doctor(region: &str, own: &str, label: &str) -> String {
    let wanted = format!("end = \"{SECTION_PREFIX}{own}\"");
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
        if !done && current == Some(region) && t == wanted {
            out.push(format!("end = \"{label}\""));
            done = true;
            continue;
        }
        out.push(line.to_string());
    }
    out.join("\n")
}

/// RED-FIRST, PERMANENTLY, AGAINST THE REAL CORPUS: for every `section:` region whose
/// successor has a head label, re-spelling its end as that label (the neighbour's
/// placement, the mis-measure R6 exists for) must make the live manifest REFUSE to
/// resolve, naming the region, the section its content ends in, and the `section:`
/// remedy. A gate nobody has seen fail proves nothing; this one fails once per target on
/// every run, on the actual layout.
///
/// Which refusal fires is layout-dependent and both are correct: a successor placed past
/// a pad reports the pad; a flush successor reports that the label is defined in another
/// section. `resolve` stops at the first, so either is accepted.
///
/// LOUD WHEN UNMEASURABLE: no target at all is a failure, not a pass.
#[test]
fn respelling_a_section_end_as_the_successor_head_refuses_the_live_manifest() {
    let Some(live) = live() else {
        return;
    };
    let (targets, candidates) = doctoring_targets(&live);
    assert!(candidates > 0, "repin.toml has no both-shape `section:` region, the control has nothing to doctor");
    assert!(
        !targets.is_empty(),
        "none of the {candidates} `section:` region(s) has a successor with a head label in both shapes: \
         the negative control measured NOTHING"
    );
    eprintln!(
        "region_end_contracts: doctoring {} of {candidates} both-shape `section:` region(s) \
         (the rest have no successor head label defined in both shapes)",
        targets.len()
    );
    for t in &targets {
        let doctored = doctor(&t.region, &t.own, &t.label);
        assert_ne!(doctored, MANIFEST, "region `{}`: nothing was re-spelled", t.region);
        let manifest = load_manifest(&doctored)
            .unwrap_or_else(|e| panic!("region `{}`: doctored manifest must still parse: {e}", t.region));
        let err = match resolve(&manifest, &live.plain, &live.debug) {
            Err(e) => e,
            Ok(_) => panic!(
                "region `{}`: the manifest resolved with `end = \"{}\"`, the head of the successor \
                 section `{}`, the strict end contract is not reaching this region",
                t.region, t.label, t.successor
            ),
        };
        assert!(err.contains(&format!("region `{}` (", t.region)), "the refusal names the region: {err}");
        assert!(
            err.contains(&format!("placer pad past section `{}`", t.own))
                || err.contains(&format!("is defined in section `{}`", t.successor)),
            "the refusal names the section the content ends in, or the label's owner: {err}"
        );
        assert!(
            err.contains(&format!("{SECTION_PREFIX}{}", t.own)),
            "the refusal names the `section:` remedy: {err}"
        );
    }
}
