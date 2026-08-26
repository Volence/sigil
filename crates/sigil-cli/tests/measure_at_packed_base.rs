//! MEASURE-AT-PACKED-BASE: upstream code growth must not brick the build through a
//! lying layout measurement.
//!
//! THE REPORT THIS REPRODUCES (2026-08-26, ring-sparkle): seven `nop`s added to
//! `RingCollision` — 14 bytes of code growth in `engine/objects/rings.emp`, far
//! upstream of `player_sensors` — made `sigil build --game sonic4` refuse with an
//! overlap naming `section`/`player_sensors`, a pair nothing had touched. The
//! packing walk's collision-fallback measurement parked `collision_data` at a
//! scratch slot that wraps the 24-bit bus (`0x300_0000`, masked to `0x0`), so its
//! tables looked abs.w-reachable and twelve `lea` sites in `player_sensors`
//! measured 4 bytes each where the real base encodes 6 — the walk packed the
//! successor 24 bytes into the section and called the result an anchor overrun.
//! The fix measures every round at its own bases through the overlap-tolerant
//! resolve (`resolve_layout_measuring`), so no substitute base exists to lie.
//!
//! WHAT THIS GATE ASSERTS. On a COPY of the reference tree (the doctoring rides
//! `shadow_aeon_tree` — a copy, because the manifest scan does not descend a
//! symlinked directory and the embed sandbox refuses a path outside the root):
//!   1. the undoctored copy builds (the copy mechanism itself is not the signal);
//!   2. with the report's exact seven `nop`s injected, the build SUCCEEDS and the
//!      image differs from the undoctored one (the growth really landed);
//!   3. with a parcel-scale growth (5000 bytes — past the old rig's spread step,
//!      which could not even measure it), the build still succeeds and the
//!      `[layout.provisional-drift]` warnings name the sections that moved.
//!
//! WHY THIS FILE NAMES NO BUILT ARTIFACT. It builds from SOURCE through the same
//! entry `sigil build` reaches and compares no byte against any committed
//! reference image — the nightly source-gate lane's self-audit files it as
//! source-only by that very absence; do not add the words the audit greps for
//! (see `scripts/nightly_source_gates.sh`).
//!
//! Reference tree: `AEON_DIR`, else the sibling aeon checkout. A missing tree
//! skips green outside strict mode and HARD-FAILS under `SIGIL_STRICT_GATE=1`.

use sigil_harness::native;
use sigil_harness::test_support::{reference_tree_for_profile, shadow_aeon_tree};
use std::path::Path;

/// The module the report's growth lands in, and the line the `nop`s go after —
/// asserted to exist exactly once so a refactor cannot silently doctor nothing.
const RINGS_MODULE: &str = "engine/objects/rings.emp";
const INJECT_AFTER: &str = "pub proc RingCollision () clobbers(d0-d7/a0-a3) {\n";

/// The sonic4 shape the report names (the plain one — the first sonic4 row in
/// [`native::shipped_shapes`], the same table every other gate enumerates).
fn sonic4_shape() -> (&'static str, native::GameProfile) {
    native::shipped_shapes()
        .into_iter()
        .find(|(_, p)| p.game_root_rel.starts_with("games/sonic4/"))
        .expect("a shipped sonic4 shape exists")
}

/// The rings module's text with `count` `nop`s injected at the top of
/// `RingCollision` — the report's exact growth shape (each `nop` is 2 bytes).
fn grown_rings(aeon: &Path, count: usize) -> String {
    let src = std::fs::read_to_string(aeon.join(RINGS_MODULE))
        .unwrap_or_else(|e| panic!("read {RINGS_MODULE}: {e}"));
    assert_eq!(
        src.matches(INJECT_AFTER).count(),
        1,
        "the injection anchor must appear exactly once in {RINGS_MODULE}"
    );
    src.replace(INJECT_AFTER, &format!("{INJECT_AFTER}{}", "        nop\n".repeat(count)))
}

/// Build one shape exactly as `sigil build` does; `Err` is the refusal or the
/// first error-level diagnostic.
fn build(aeon: &Path, label: &str, profile: &native::GameProfile) -> Result<native::RomBuild, String> {
    let built = native::build_rom_chained_with_listing(aeon, profile)
        .map_err(|e| format!("shape `{label}`: {e}"))?;
    if let Some(first) =
        built.warnings.iter().find(|w| w.level == sigil_span::Level::Error)
    {
        return Err(format!("shape `{label}`: built with an error-level diagnostic: {}", first.message));
    }
    if built.rom.is_empty() {
        return Err(format!("shape `{label}`: built an EMPTY image"));
    }
    Ok(built)
}

/// THE REPRODUCTION: the report's seven `nop`s (14 bytes upstream of
/// `player_sensors`) build, and the undoctored copy proves the mechanism.
#[test]
fn fourteen_bytes_of_upstream_code_growth_still_builds() {
    let (label, profile) = sonic4_shape();
    let Some(aeon) = reference_tree_for_profile(&profile) else { return };

    let clean = shadow_aeon_tree(&aeon, &[]).expect("shadow copy of the reference tree");
    let clean_rom = build(clean.root(), label, &profile)
        .unwrap_or_else(|e| panic!("the UNDOCTORED copy must build (the copy is not the signal): {e}"))
        .rom;

    let doctored = grown_rings(&aeon, 7);
    let grown = shadow_aeon_tree(&aeon, &[(RINGS_MODULE, &doctored)])
        .expect("shadow copy with the injected growth");
    let grown_rom = build(grown.root(), label, &profile).unwrap_or_else(|e| {
        panic!(
            "14 bytes of upstream code growth must BUILD — the layout walk measured a \
             section at a substitute base and refused an innocent pair: {e}"
        )
    })
    .rom;

    assert_ne!(clean_rom, grown_rom, "the injected growth must actually reach the image");
}

/// PARCEL-SCALE GROWTH: 2500 `nop`s (5000 bytes) — more than the pre-fix measuring
/// rig could even measure (its spread stepped 0x400 per section) — builds, packs the
/// downstream run, and reports `[layout.provisional-drift]` naming what moved.
#[test]
fn parcel_scale_growth_packs_downstream_and_reports_the_drift() {
    let (label, profile) = sonic4_shape();
    let Some(aeon) = reference_tree_for_profile(&profile) else { return };

    let doctored = grown_rings(&aeon, 2500);
    let grown = shadow_aeon_tree(&aeon, &[(RINGS_MODULE, &doctored)])
        .expect("shadow copy with the parcel-scale growth");
    let built = build(grown.root(), label, &profile)
        .unwrap_or_else(|e| panic!("5000 bytes of code growth must build and pack downstream: {e}"));

    let drift: Vec<&native::BuildWarning> =
        built.warnings.iter().filter(|w| w.id == "layout.provisional-drift").collect();
    assert!(
        !drift.is_empty(),
        "a growth past the drift tolerance must be reported, not silent"
    );
    assert!(
        drift.iter().any(|w| w.message.contains("refreeze at landing")),
        "the drift report says what the landing owes: {:?}",
        drift.first().map(|w| &w.message)
    );
}
