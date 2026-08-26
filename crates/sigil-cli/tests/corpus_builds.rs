//! CORPUS-BUILDS: every shipped shape still BUILDS from aeon SOURCE.
//!
//! The brick witness for the nightly source-gate lane. A BRICK is the compiler
//! refusing the corpus — `[map.order-undeclared]`, `section … has no region in the
//! map`, colliding pins, an unknown function in a reached module — and it is a
//! SOURCE-ONLY fact: `sigil build` compiles aeon source into a ROM, so whether the
//! seven shipped shapes build is measurable on a checkout that has never run
//! `./build.sh`. A brick is a different object from CRC DRIFT (a legitimate byte move
//! awaiting sigil's refreeze): drift is red by design between an aeon parcel and the
//! refreeze and belongs to the artifact lane; a brick is red until someone fixes it
//! and belongs here.
//!
//! WHAT THIS GATE ASSERTS. For each shape in [`native::shipped_shapes`] — the ONE
//! table meaning "all shipped shapes", the same one the byte gates enumerate — the
//! build entry `sigil build` reaches (`build_rom_chained_with_listing`; the canonical
//! sonic4 driver delegates to it) returns `Ok` and its diagnostic channel carries no
//! error-level row. Every shape is measured before any is judged, so a run names
//! EVERY bricked shape, not the first.
//!
//! WHAT IT DOES NOT ASSERT, ON PURPOSE. No byte is compared against any committed
//! reference image — that is the artifact lane's job, and a comparison here would
//! make this gate red across every byte-moving aeon parcel, which is exactly the
//! window the nightly must NOT report. The deb2 symbol appendix is not appended
//! either: it shells an external tool over the finished image and is not part of
//! "the compiler accepts the corpus".
//!
//! WHY THIS FILE NAMES NO BUILT ARTIFACT. The nightly lane's self-audit sorts every
//! aeon-reading test file by its own text: a file that names a built ROM image, a
//! listing file extension, or the committed reference blobs is filed as
//! artifact-lane and NOT run nightly. This gate must run nightly, so it deliberately
//! describes those inputs without spelling them — do not add the words the audit
//! greps for (see `scripts/nightly_source_gates.sh`).
//!
//! Reference tree: `AEON_DIR`, else the sibling aeon checkout. A missing tree skips
//! green outside strict mode and HARD-FAILS naming the absent path under
//! `SIGIL_STRICT_GATE=1` — the mode the nightly and the pre-merge run set.

use sigil_harness::map_placement;
use sigil_harness::native;
use sigil_harness::test_support::{reference_tree_for_profile, shadow_aeon_tree};
use std::path::Path;

/// Build one shape exactly as `sigil build` does, judging only whether the compiler
/// ACCEPTED the corpus: `Err` names the shape and the refusal, or the first
/// error-level diagnostic the build carried out.
fn build_shape(aeon: &Path, label: &str, profile: &native::GameProfile) -> Result<(), String> {
    let built = native::build_rom_chained_with_listing(aeon, profile)
        .map_err(|e| format!("shape `{label}`: {e}"))?;
    let errors: Vec<&native::BuildWarning> =
        built.warnings.iter().filter(|w| w.level == sigil_span::Level::Error).collect();
    if let Some(first) = errors.first() {
        return Err(format!(
            "shape `{label}`: built, but {} error-level diagnostic(s); first: {} at {}",
            errors.len(),
            first.message,
            first.location.as_deref().unwrap_or("<no location>")
        ));
    }
    if built.rom.is_empty() {
        return Err(format!("shape `{label}`: built an EMPTY image"));
    }
    Ok(())
}

/// One line per shape that does not build, over EVERY shipped shape. Empty means the
/// whole corpus builds.
fn corpus_bricks<'a>(
    aeon: &Path,
    shapes: impl IntoIterator<Item = (&'a str, &'a native::GameProfile)>,
) -> Vec<String> {
    shapes
        .into_iter()
        .filter_map(|(label, profile)| build_shape(aeon, label, profile).err())
        .collect()
}

/// THE GATE: every shipped shape builds from source with zero error diagnostics.
#[test]
fn every_shipped_shape_builds_from_source() {
    let shapes = native::shipped_shapes();
    assert!(!shapes.is_empty(), "shipped_shapes() enumerated nothing — a gate over no shapes gates nothing");

    // The guard is derived from each shape's own profile (its residual root and its
    // placement map), so it cannot name an input the build does not read. Under
    // strict mode a missing path panics naming itself inside `reference_tree`.
    let mut aeon = None;
    for (_, profile) in &shapes {
        let Some(root) = reference_tree_for_profile(profile) else { return };
        aeon = Some(root);
    }
    let aeon = aeon.expect("shapes is non-empty, so the loop ran at least once");

    let bricks = corpus_bricks(&aeon, shapes.iter().map(|(l, p)| (*l, p)));
    eprintln!(
        "corpus builds: {} of {} shipped shapes build from source at {}",
        shapes.len() - bricks.len(),
        shapes.len(),
        aeon.display()
    );
    assert!(
        bricks.is_empty(),
        "{} of {} shipped shapes do NOT build from aeon source (a BRICK — the compiler \
         refuses the corpus; no refreeze clears this):\n  {}",
        bricks.len(),
        shapes.len(),
        bricks.join("\n  ")
    );
}

/// The section whose placement-map row the witness below deletes: the editor-scene
/// block, declared by its SECTION NAME because its head label is content-derived.
/// Only the name is spelled here; the row it resolves to is derived through the one
/// printer the packer uses, and the witness refuses to run unless the shipped map
/// carries exactly that row.
const EDITOR_SCENE_SECTION: &str = "ojz_effects_editor_act1";

/// RED-FIRST WITNESS: a brick injected into a COPY of the tree makes the SAME checker
/// red, naming the shape and the section's head label — so a green from the gate
/// above is a measurement, not an absence of detection.
///
/// The injected brick is the one the 2026-08-26 live tree produced: the placement
/// map's `"section:ojz_effects_editor_act1"` row deleted while the block emits bytes,
/// which the map-driven packer refuses as `[map.order-undeclared]`. The doctoring
/// rides `shadow_aeon_tree` — a copy, because the manifest scan does not descend a
/// symlinked directory and the embed sandbox refuses a path outside the root.
///
/// Both directions are measured on the same copy mechanism: the undoctored shape
/// builds (so the copy itself is not what makes the doctored one red).
#[test]
fn a_deleted_map_row_bricks_the_build_and_the_gate_names_it() {
    let shapes = native::shipped_shapes();
    // The first shape whose placement map is sonic4's — the map the row lives in.
    let (label, profile) = shapes
        .iter()
        .find(|(_, p)| p.game_root_rel.starts_with("games/sonic4/"))
        .expect("a shipped sonic4 shape exists");
    let Some(aeon) = reference_tree_for_profile(profile) else { return };

    let map_path = profile.map_path(&aeon);
    let map_rel = map_path.strip_prefix(&aeon).expect("map path is under the tree");
    let map_rel = map_rel.to_str().expect("map path is UTF-8");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", map_path.display()));
    let row_key = map_placement::section_row_key(EDITOR_SCENE_SECTION);
    let row_lines: Vec<&str> =
        map_src.lines().filter(|l| l.trim().trim_matches(',').trim_matches('"') == row_key).collect();
    assert_eq!(
        row_lines.len(),
        1,
        "the shipped map must carry exactly one `{row_key}` row for this witness to delete; found {}",
        row_lines.len()
    );
    let doctored: String = map_src
        .lines()
        .filter(|l| *l != row_lines[0])
        .map(|l| format!("{l}\n"))
        .collect();

    // Direction 1: the copy mechanism alone leaves the shape green.
    let clean = shadow_aeon_tree(&aeon, &[(map_rel, &map_src)])
        .unwrap_or_else(|e| panic!("shadow aeon tree (clean): {e}"));
    build_shape(clean.root(), label, profile)
        .unwrap_or_else(|e| panic!("the undoctored copy must build: {e}"));
    drop(clean);

    // Direction 2: the deleted row bricks the build, and the report names it.
    let bricked = shadow_aeon_tree(&aeon, &[(map_rel, &doctored)])
        .unwrap_or_else(|e| panic!("shadow aeon tree (bricked): {e}"));
    let bricks = corpus_bricks(bricked.root(), [(*label, profile)]);
    assert_eq!(bricks.len(), 1, "exactly the doctored shape is red: {bricks:?}");
    let report = &bricks[0];
    eprintln!("injected brick, as the gate reports it:\n  {report}");
    assert!(
        report.contains("[map.order-undeclared]"),
        "the deleted row must surface as the map-completeness refusal, got: {report}"
    );
    assert!(
        report.contains(&format!("shape `{label}`")),
        "the report must name the shape, got: {report}"
    );
}
