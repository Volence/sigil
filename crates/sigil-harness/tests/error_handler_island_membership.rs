//! THE SHAPES THAT DECLARE THE MD DEBUGGER ISLAND ARE EXACTLY THE SHAPES WHOSE
//! BUILDS DEFINE ITS BLOB LABEL — asserted per shape, in both directions.
//!
//! `native::check_error_handler_is_last` measures the deb2 appendix against
//! `ErrorHandlerBlob + ERROR_HANDLER_BLOB_LEN`, and it now refuses a shape that
//! declares the island but produces no such label (see
//! `error_handler_island_order.rs`). That closes the direction it can see. It cannot
//! see the other one: the `lean` shape never reaches it — its callers return the
//! assembled image before any appendix is considered — so a shape that is not supposed
//! to carry the island could grow one and no build-time check would look. And no
//! single build can tell you whether the SET of island-carrying shapes is the set the
//! tree declares, because each build sees only itself.
//!
//! So this walks every shape in `native::shipped_shapes` — the one table meaning "all
//! shipped shapes" — and set-diffs two answers per shape.
//!
//! WHY THE EXPECTATION IS NOT CIRCULAR. An expectation read off the same build output
//! it judges asserts that the output agrees with itself, which is worth nothing. The
//! declared answer here comes from `native::declares_error_handler_island`, which
//! reads no build output at all: it reconciles the profile's `debug || crash_report`
//! axis with whether `profile.registry` carries `engine.debug.error_handler`. The
//! registry is the module list the build is HANDED — upstream of every section, label
//! and listing row it goes on to produce, and downstream of nothing it decides. The
//! two are independent enough to disagree (a profile may build its registry from a
//! different `(debug, crash_report)` pair than it stores, and `config_b_profile` and
//! `lean_profile` both call `registry()` with explicit arguments), which is why that
//! function reports a disagreement rather than picking a winner.
//!
//! WHAT THIS CANNOT CATCH, stated rather than implied. A change that removes the
//! registry row AND clears `crash_report` for the same shape moves both answers
//! together, and this gate follows it green — correctly, because that is a coherent
//! redeclaration of what the shape IS, not a drift between two records of it. What it
//! refuses is any change to one record alone: a renamed blob label, an island that
//! stopped being placed, a harvest that lost the row, a registry re-gate that forgot
//! the profile field, or a lean-like shape that grew an island.

use sigil_harness::native::{self, GameProfile};
use sigil_harness::test_support::{reference_tree_for_profile, shadow_aeon_tree};
use sigil_link::ListingSymbol;
use std::path::Path;

/// THE OBSERVATION. Read exactly the way the build-time guard reads it, so this gate
/// and that guard cannot disagree about what "the shape emits the island" means.
fn listing_carries_island(listing: &[ListingSymbol]) -> bool {
    listing.iter().any(|l| l.name == native::ERROR_HANDLER_BLOB_LABEL)
}

/// One shape's two answers plus the size of the population the observation was made
/// over — a listing that collapsed to a handful of rows would make `emitted` false for
/// a reason that has nothing to do with the island, and the failure text says so.
struct Shape {
    label: String,
    declared: bool,
    emitted: bool,
    symbols: usize,
}

/// THE SET DIFF, both directions, one line per shape that disagrees with itself.
/// A pure function over measured rows, so the two witnesses below can drive it with
/// rows they doctored and see it name the shape.
fn membership_faults(rows: &[Shape]) -> Vec<String> {
    rows.iter()
        .filter(|r| r.declared != r.emitted)
        .map(|r| {
            if r.declared {
                format!(
                    "shape `{}`: DECLARES the error_handler island, but its build \
                     defines no `{}` among {} listing symbol(s). The island is not \
                     placed, its blob label was renamed, or the harvest lost the row — \
                     each of those ships a ROM whose crash screen prints `<unknown>` \
                     for every Offset/Caller and reports nothing else.",
                    r.label,
                    native::ERROR_HANDLER_BLOB_LABEL,
                    r.symbols,
                )
            } else {
                format!(
                    "shape `{}`: declares NO error_handler island, yet its build \
                     defines `{}` among {} listing symbol(s). The island and the lean \
                     fault handler are the two arms of one registry `if` sharing a \
                     placement slot, so this shape has lost the split — and it takes \
                     the no-appendix path, so the blob it now carries would find no \
                     symbol table at all.",
                    r.label,
                    native::ERROR_HANDLER_BLOB_LABEL,
                    r.symbols,
                )
            }
        })
        .collect()
}

/// Measure one shape: its declared answer (no build output read) and its emitted
/// answer (its own build's listing). Every shape is measured before any is judged, so
/// a run names every disagreeing shape rather than the first.
fn measure(aeon: &Path, label: &str, profile: &GameProfile) -> Result<Shape, String> {
    let declared = native::declares_error_handler_island(profile)?;
    let built = native::build_rom_chained_with_listing(aeon, profile)
        .map_err(|e| format!("shape `{label}`: build failed, so nothing was measured: {e}"))?;
    Ok(Shape {
        label: label.to_string(),
        declared,
        emitted: listing_carries_island(&built.listing),
        symbols: built.listing.len(),
    })
}

/// THE GATE.
#[test]
fn every_shipped_shape_emits_the_island_label_exactly_when_it_declares_one() {
    let shapes = native::shipped_shapes();
    assert!(
        !shapes.is_empty(),
        "shipped_shapes() enumerated nothing — a set diff over an empty set is green \
         for the one reason that proves nothing"
    );

    // The guard is derived from each shape's own profile (its residual root and its
    // placement map). Strict mode turns a reference it cannot find into a panic naming
    // the path, so this cannot report green without having built anything.
    let mut aeon = None;
    for (_, profile) in &shapes {
        let Some(root) = reference_tree_for_profile(profile) else { return };
        aeon = Some(root);
    }
    let aeon = aeon.expect("shapes is non-empty, so the loop ran at least once");

    let mut rows: Vec<Shape> = Vec::new();
    let mut unmeasured: Vec<String> = Vec::new();
    for (label, profile) in &shapes {
        match measure(&aeon, label, profile) {
            Ok(row) => rows.push(row),
            Err(e) => unmeasured.push(e),
        }
    }

    // LOUD ON UNMEASURABLE: a shape that could not be measured is not a shape that
    // agreed. It is named and it fails, never folded into the green count.
    assert!(
        unmeasured.is_empty(),
        "{} of {} shipped shapes could not be measured, so this run says nothing about \
         them:\n  {}",
        unmeasured.len(),
        shapes.len(),
        unmeasured.join("\n  "),
    );

    // BOTH DIRECTIONS NEED A POPULATION. With every shape on one side of the split,
    // one half of the diff is enforcing a rule over nothing while reading as coverage
    // — which is the same defect class this file exists to close, one level up.
    let carrying = rows.iter().filter(|r| r.declared).count();
    let bare = rows.len() - carrying;
    assert!(
        carrying > 0,
        "no shipped shape declares the error_handler island, so the declares-it-must-\
         emit-it direction is enforced over an empty set. Either the shape table lost \
         its island-carrying shapes or the declaration stopped being read."
    );
    assert!(
        bare > 0,
        "every shipped shape declares the error_handler island, so the declares-none-\
         must-emit-none direction is enforced over an empty set. The `lean` profile is \
         the shape that gives that direction a subject; if it is gone from the table, \
         this gate is half of what it reads as."
    );

    let faults = membership_faults(&rows);
    eprintln!(
        "island membership: {} of {} shipped shapes declare the MD Debugger island \
         ({carrying} carrying / {bare} bare), and {} of them agree with their own build",
        carrying,
        rows.len(),
        rows.len() - faults.len(),
    );
    assert!(
        faults.is_empty(),
        "{} of {} shipped shapes disagree with their own declaration:\n  {}",
        faults.len(),
        rows.len(),
        faults.join("\n  "),
    );
}

/// The `.emp` source that declares the blob, and the label declaration this witness
/// renames. Spelled as the module's own path so the doctoring names a real file: the
/// shadow helper refuses an override that matches nothing.
const ISLAND_SOURCE_REL: &str = "engine/debug/error_handler.emp";

/// RED-FIRST, END TO END, AND THE DEFECT THIS GATE EXISTS FOR: the blob label renamed
/// in a COPY of the engine tree.
///
/// This is the shape of the failure the old vacuous arm could not see — the island is
/// still placed, the shape still declares it, everything still builds, and the only
/// difference is that the name the debugger's consumers look for is no longer the name
/// the source defines. The rename covers every occurrence in the module, so the `pub
/// equ MDDBG__*` table still derives off the label it defines and the build stays
/// clean; what changes is the one row this gate reads.
///
/// A demo shape carries the witness: it is engine-only, so the doctoring reaches it
/// through the same shared module every game's build reads, and it is sound-OFF, so no
/// generated audio has to exist in the copy.
#[test]
fn renaming_the_blob_label_in_the_engine_tree_makes_the_gate_red() {
    let profile = native::demo_profile(true);
    let Some(aeon) = reference_tree_for_profile(&profile) else { return };
    assert!(
        native::declares_error_handler_island(&profile).expect("the demo debug shape is coherent"),
        "control: this witness needs a shape that DECLARES the island"
    );

    // The control half, on the undoctored tree: the copy mechanism is not what makes
    // the doctored measurement red.
    let clean = native::build_rom_chained_with_listing(&aeon, &profile)
        .unwrap_or_else(|e| panic!("undoctored build: {e}"));
    assert!(
        listing_carries_island(&clean.listing),
        "control: the shape declares the island, so its own build defines the blob label"
    );

    let src = std::fs::read_to_string(aeon.join(ISLAND_SOURCE_REL))
        .unwrap_or_else(|e| panic!("read {ISLAND_SOURCE_REL}: {e}"));
    let renamed = format!("{}Renamed", native::ERROR_HANDLER_BLOB_LABEL);
    let doctored_src = src.replace(native::ERROR_HANDLER_BLOB_LABEL, &renamed);
    assert_ne!(
        doctored_src, src,
        "the label must occur in {ISLAND_SOURCE_REL}, or this witness doctored nothing"
    );
    let shadow = shadow_aeon_tree(&aeon, &[(ISLAND_SOURCE_REL, &doctored_src)])
        .unwrap_or_else(|e| panic!("shadow tree: {e}"));

    let row = measure(shadow.root(), "demo debug", &profile)
        .unwrap_or_else(|e| panic!("the doctored tree must still BUILD — a build failure \
                                    would make this witness prove something else: {e}"));
    assert!(row.declared, "the profile is untouched, so it still declares the island");
    assert!(!row.emitted, "the label was renamed, so the old name is defined nowhere");

    let faults = membership_faults(&[row]);
    assert_eq!(faults.len(), 1, "the doctored shape must be the one fault: {faults:?}");
    assert!(
        faults[0].contains("demo debug"),
        "the fault must name the shape; got: {}",
        faults[0]
    );
    assert!(
        faults[0].contains("<unknown>"),
        "the declares-but-does-not-emit direction must name the silent runtime symptom, \
         because nothing else in the build reports it; got: {}",
        faults[0]
    );
}

/// RED-FIRST, THE DECLARED SIDE, and the other direction of the diff: a shape whose
/// profile says it carries no island, judged against a build that does, is named. Both
/// halves are doctored off a real shape, so neither the declaration nor the observation
/// is a constant this file wrote down.
#[test]
fn a_shape_that_declares_no_island_but_emits_one_is_named() {
    let shapes = native::shipped_shapes();
    let (label, profile) = shapes
        .iter()
        .find(|(_, p)| native::declares_error_handler_island(p).unwrap_or(false))
        .expect("a shipped shape declares the island");
    let Some(aeon) = reference_tree_for_profile(profile) else { return };

    let built = native::build_rom_chained_with_listing(&aeon, profile)
        .unwrap_or_else(|e| panic!("shape `{label}`: {e}"));

    let rows = vec![Shape {
        label: label.to_string(),
        declared: false,
        emitted: listing_carries_island(&built.listing),
        symbols: built.listing.len(),
    }];
    let faults = membership_faults(&rows);
    assert_eq!(faults.len(), 1, "the mis-declared shape must be the one fault: {faults:?}");
    assert!(
        faults[0].contains("lost the split"),
        "the declares-none-but-emits direction is a different fault from the other and \
         must read as one; got: {}",
        faults[0]
    );
}

/// The declared answer is a RECONCILIATION of two records, and this proves it is live
/// in both of its failure modes. Neither probe touches the tree: they doctor a shipped
/// profile in memory.
#[test]
fn the_declared_answer_refuses_a_profile_whose_two_records_disagree() {
    // A profile that keeps the crash-report axis but loses the registry row: the two
    // records now say different things about one shape, and neither is authoritative
    // over the other.
    let mut half_removed = native::sonic4_profile(false);
    assert!(half_removed.crash_report, "control: this profile's axis says it carries the island");
    half_removed.registry.retain(|m| m.module_id != native::ERROR_HANDLER_MODULE_ID);
    let e = native::declares_error_handler_island(&half_removed)
        .expect_err("axis says island, registry places neither handler");
    assert!(
        e.contains("EXCLUSIVE"),
        "removing one arm leaves the split with NEITHER arm, which is the first thing \
         that stops making sense; got: {e}"
    );

    // Both arms placed: the split is gone the other way. The second arm is minted off
    // the island's OWN spec, which is also the truth it models — the two handlers share
    // one placement slot.
    let mut both_arms = native::sonic4_profile(false);
    let slot = both_arms
        .registry
        .iter()
        .find(|m| m.module_id == native::ERROR_HANDLER_MODULE_ID)
        .expect("control: the island is in this profile's registry")
        .region;
    both_arms.registry.push(native::ModuleSpec {
        module_id: native::RELEASE_FAULT_MODULE_ID,
        section: "release_fault",
        region: slot,
    });
    let e = native::declares_error_handler_island(&both_arms)
        .expect_err("a registry placing both fault handlers has lost the split");
    assert!(e.contains("BOTH"), "must say which way the split broke; got: {e}");

    // The split intact but the axis cleared: the two records disagree, and this is the
    // arm that catches a registry re-gate that forgot the profile field.
    let mut axis_cleared = native::lean_profile();
    assert!(
        !native::declares_error_handler_island(&axis_cleared).expect("lean is coherent"),
        "control: the lean profile declares no island"
    );
    axis_cleared.crash_report = true;
    let e = native::declares_error_handler_island(&axis_cleared)
        .expect_err("axis says island, registry places the lean handler");
    assert!(
        e.contains("AXIS and the REGISTRY disagree"),
        "must name the disagreement rather than picking a winner; got: {e}"
    );
}
