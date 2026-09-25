//! THE LISTING'S HARVESTED ENGINE-CONSTANT ROWS DESCRIBE THE ROM THEY SHIP WITH, IN
//! THE SHAPE THAT BUILT IT.
//!
//! `engine/system/constants.emp`'s `pub const`s are harvested and published as `EQU`
//! rows in the listing (guarded define, then link `EquSym`, then `resolved_equates`,
//! then `emit_listing`). The `.emp` lowering that encodes the same constants into the
//! ROM folds them under the shape's define set. A `pub const` that reads a build
//! define (`PAGE_FRAMES_CLAMP` reads `STRESS_EVICT`) therefore has two values unless
//! the harvest folds under the SAME define set, and the listing would publish one
//! while the ROM carries the other.
//!
//! The one shape where the two can differ today is `stress_evict` (`STRESS_EVICT=1`):
//! at `STRESS_EVICT=0` both folds evaluate the same expression. So this gate builds
//! that shape and charges its published listing with three agreements, every expected
//! value DERIVED, none transcribed:
//!
//!  1. `PAGE_FRAMES_CLAMP`'s published `EQU` value equals the immediate the ROM
//!     actually encodes at its one use site (`cmpi.w #PAGE_FRAMES_CLAMP, d6` inside
//!     `Level_LoadArt`, read out of the built image);
//!  2. that immediate equals the fold of `constants.emp` under the profile's own
//!     define set (`shape_defines`), so the ROM half and the define half agree;
//!  3. every other harvested engine constant that has an `EQU` row publishes the
//!     fold under the same define set.
//!
//! NON-VACUITY: the stress fold must DIFFER from the canonical debug shape's fold for
//! `PAGE_FRAMES_CLAMP`. If the fixture ever goes inert (the two folds coincide), a
//! shape-blind harvest would pass this gate for the wrong reason, so the gate fails
//! and says so instead.
//!
//! UNMEASURABLE IS RED: a build failure, a missing row, a missing label, or a use site
//! that cannot be identified uniquely fails the test naming what was not measured.

use sigil_harness::native::{self, GameProfile};
use sigil_harness::test_support::reference_tree_for_profile;
use std::collections::HashMap;
use std::path::Path;

/// The constant whose fold reads a build define, and the routine that encodes it.
const CLAMP: &str = "PAGE_FRAMES_CLAMP";
const USE_SITE: &str = "Level_LoadArt";

/// `cmpi.w #<imm16>, d6`: opcode word `$0C46` (CMPI, size word, EA mode 0 reg 6).
const CMPI_W_IMM_D6: u16 = 0x0C46;

/// Fold every `pub const` of `engine/system/constants.emp` under `profile`'s own
/// define set, independently of the harvest under test: a direct parse + fold with
/// `shape_defines`, the define set the `.emp` build lowers under.
fn fold_under_profile(aeon: &Path, profile: &GameProfile) -> HashMap<String, i64> {
    let path = aeon.join("engine/system/constants.emp");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("UNMEASURED: read {}: {e}", path.display()));
    let (file, pdiags) = sigil_frontend_emp::parse_str(&src);
    assert!(
        !pdiags.iter().any(|d| d.level == sigil_span::Level::Error),
        "UNMEASURED: {} does not parse: {:?}",
        path.display(),
        pdiags.first()
    );
    let defines = native::shape_defines(profile, aeon)
        .unwrap_or_else(|e| panic!("UNMEASURED: shape `{}` define env: {e}", profile.name));
    let (vals, diags) =
        sigil_frontend_emp::eval::eval_all_pub_consts(&file, Some(aeon), &defines);
    assert!(
        !diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "UNMEASURED: folding {} under shape `{}`'s defines failed: {:?}",
        path.display(),
        profile.name,
        diags.first()
    );
    vals.into_iter().collect()
}

/// The published `EQU` rows of a listing text, name to value.
fn published_equates(text: &str) -> HashMap<String, u32> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("EQU ") else { continue };
        let Some((name, val)) = rest.split_once(" = $") else { continue };
        let v = u32::from_str_radix(val.trim(), 16)
            .unwrap_or_else(|e| panic!("unparseable EQU row `{line}`: {e}"));
        out.insert(name.to_string(), v);
    }
    out
}

/// Every `cmpi.w #imm, d6` immediate inside `[start, end)` of the image.
fn cmpi_d6_immediates(rom: &[u8], start: usize, end: usize) -> Vec<(usize, u16)> {
    let word = |at: usize| u16::from_be_bytes([rom[at], rom[at + 1]]);
    let mut out = Vec::new();
    let mut at = start;
    while at + 4 <= end.min(rom.len()) {
        if word(at) == CMPI_W_IMM_D6 {
            out.push((at, word(at + 2)));
        }
        at += 2;
    }
    out
}

#[test]
fn stress_shape_listing_publishes_the_harvested_constant_the_rom_encodes() {
    let profile = native::stress_evict_profile();
    let Some(aeon) = reference_tree_for_profile(&profile) else { return };

    let build = native::build_rom_chained_with_listing(&aeon, &profile).unwrap_or_else(|e| {
        panic!("UNMEASURED: shape `{}` did not build, so nothing was measured: {e}", profile.name)
    });
    // The listing TEXT, as `sigil build --emit-lst` publishes it (minus the digest
    // header, which carries no equate rows).
    let text = sigil_link::emit_listing(&build.listing);
    let published = published_equates(&text);
    assert!(!published.is_empty(), "UNMEASURED: the stress listing publishes no EQU rows");

    // Expectations, derived.
    let stress = fold_under_profile(&aeon, &profile);
    let canonical = fold_under_profile(&aeon, &native::sonic4_profile(true));
    let want_clamp = *stress
        .get(CLAMP)
        .unwrap_or_else(|| panic!("UNMEASURED: constants.emp has no pub const `{CLAMP}`"));
    let canonical_clamp = canonical[CLAMP];
    assert_ne!(
        want_clamp, canonical_clamp,
        "NON-VACUOUS CONTROL FAILED: `{CLAMP}` folds to {want_clamp} in shape `{}` and in \
         the canonical debug shape alike, so this gate cannot tell a shape-aware harvest \
         from a shape-blind one. The fixture define went inert; re-point the gate at a \
         constant the shape actually moves",
        profile.name
    );

    // 1. The ROM's encoded immediate at the one use site.
    let labels: Vec<(&str, u32)> = build
        .listing
        .iter()
        .filter(|s| !s.is_equate)
        .map(|s| (s.name.as_str(), s.value))
        .collect();
    let start = labels
        .iter()
        .find(|(n, _)| *n == USE_SITE)
        .map(|(_, v)| *v)
        .unwrap_or_else(|| panic!("UNMEASURED: no `{USE_SITE}` label in the stress listing"));
    // The routine's extent: up to the next label above it that is not one of its own
    // locals (a demangled or mangled local carries the parent's name).
    let end = labels
        .iter()
        .filter(|(n, v)| *v > start && !n.contains(USE_SITE))
        .map(|(_, v)| *v)
        .min()
        .unwrap_or_else(|| panic!("UNMEASURED: no label follows `{USE_SITE}`"));
    let sites = cmpi_d6_immediates(&build.rom, start as usize, end as usize);
    assert_eq!(
        sites.len(),
        1,
        "UNMEASURED: expected exactly one `cmpi.w #imm, d6` in `{USE_SITE}` \
         [${start:06X}, ${end:06X}), found {sites:X?}. The use site cannot be identified"
    );
    let (site_at, rom_imm) = sites[0];

    // 2. The ROM agrees with the fold under the profile's defines.
    assert_eq!(
        i64::from(rom_imm),
        want_clamp,
        "the ROM encodes `cmpi.w #${rom_imm:04X}, d6` at ${site_at:06X} but `{CLAMP}` folds \
         to {want_clamp} under shape `{}`'s defines: the derivation does not describe this \
         build",
        profile.name
    );

    // 1 (the assertion). The published row equals what the ROM encodes.
    let got = *published
        .get(CLAMP)
        .unwrap_or_else(|| panic!("UNMEASURED: the stress listing publishes no `EQU {CLAMP}` row"));
    assert_eq!(
        got,
        u32::from(rom_imm),
        "LISTING-EQU-SHAPE-BLIND: the stress listing publishes `EQU {CLAMP} = ${got:08X}` \
         but the ROM from the same build encodes `cmpi.w #${rom_imm:04X}, d6` at \
         ${site_at:06X}. (The canonical fold is {canonical_clamp}; a published value equal \
         to it is the shape-blind harvest.)"
    );

    // 3. Every harvested engine constant with an EQU row publishes this shape's fold.
    let mut compared = 0usize;
    let mut faults = Vec::new();
    for (name, value) in &stress {
        let Some(got) = published.get(name) else { continue };
        compared += 1;
        let want = *value as u32;
        if *got != want {
            faults.push(format!("`{name}`: published ${got:08X}, shape fold ${want:08X}"));
        }
    }
    assert!(
        compared > 1,
        "UNMEASURED: only {compared} harvested engine constant(s) have EQU rows in the \
         stress listing, so the whole-module comparison measured nothing"
    );
    assert!(
        faults.is_empty(),
        "{} of {compared} harvested engine constants publish a value other than their fold \
         under shape `{}`'s defines:\n  {}",
        faults.len(),
        profile.name,
        faults.join("\n  ")
    );
}
