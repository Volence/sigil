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
//! So this walks every shape in `shapes::shipped` — every non-fixture `[shapes.*]` row
//! the tree describes, the one set meaning "all shipped shapes" — and set-diffs two
//! answers per shape.
//!
//! WHY THE EXPECTATION IS NOT CIRCULAR. An expectation read off the same build output
//! it judges asserts that the output agrees with itself, which is worth nothing. The
//! declared answer here comes from `sigil_build::driver::declares_island`, which reads
//! no build output at all: it reconciles the shape's `debug || crash_report` features
//! with whether the shape's module roots (the lowered program's registry, derived from
//! the game's map through the shape gates, before anything is placed or linked) carry
//! the engine's `error_handler` module. The roots are the module list the build lowers,
//! upstream of every section, label and listing row it goes on to produce. The two are independent enough to disagree (a
//! map that lost the island's row, or a tree that lost the module, derives roots
//! without it while the axis still says island), which is why that function reports a
//! disagreement rather than picking a winner.
//!
//! WHAT THIS CANNOT CATCH, stated rather than implied. A change that removes the
//! registry row AND clears `crash_report` for the same shape moves both answers
//! together, and this gate follows it green — correctly, because that is a coherent
//! redeclaration of what the shape IS, not a drift between two records of it. What it
//! refuses is any change to one record alone: a renamed blob label, an island that
//! stopped being placed, a harvest that lost the row, a registry re-gate that forgot
//! the shape's features, or a lean-like shape that grew an island.

use sigil_build::describe::ResolvedShape;
use sigil_build::driver::{declares_island, Project};
use sigil_build::place::ModuleSpec;
use sigil_harness::shapes;
use sigil_harness::test_support::{reference_tree_for_shape, shadow_aeon_tree};
use sigil_link::ListingSymbol;
use std::path::Path;

/// The engine's blob label (`engine.toml` `error_handler_blob`), the label the deb2
/// appendix measures against.
fn blob_label(p: &Project) -> &str {
    &p.engine.error_handler_blob
}

/// THE OBSERVATION. Read exactly the way the build-time guard reads it, so this gate
/// and that guard cannot disagree about what "the shape emits the island" means.
fn listing_carries_island(listing: &[ListingSymbol], label: &str) -> bool {
    listing.iter().any(|l| l.name == label)
}

/// One shape's two answers plus the size of the population the observation was made
/// over — a listing that collapsed to a handful of rows would make `emitted` false for
/// a reason that has nothing to do with the island, and the failure text says so.
struct Row {
    label: String,
    blob: String,
    declared: bool,
    emitted: bool,
    symbols: usize,
}

/// THE SET DIFF, both directions, one line per shape that disagrees with itself.
/// A pure function over measured rows, so the two witnesses below can drive it with
/// rows they doctored and see it name the shape.
fn membership_faults(rows: &[Row]) -> Vec<String> {
    rows.iter()
        .filter(|r| r.declared != r.emitted)
        .map(|r| {
            if r.declared {
                format!(
                    "shape `{}`: DECLARES the error_handler island, but its build \
                     defines no `{}` among {} listing symbol(s). The island is not \
                     placed, its blob label was renamed, or the harvest lost the row, \
                     each of those ships a ROM whose crash screen prints `<unknown>` \
                     for every Offset/Caller and reports nothing else.",
                    r.label, r.blob, r.symbols,
                )
            } else {
                format!(
                    "shape `{}`: declares NO error_handler island, yet its build \
                     defines `{}` among {} listing symbol(s). The island and the lean \
                     fault handler are the two arms of one shape gate sharing a \
                     placement slot, so this shape has lost the split, and it takes \
                     the no-appendix path, so the blob it now carries would find no \
                     symbol table at all.",
                    r.label, r.blob, r.symbols,
                )
            }
        })
        .collect()
}

/// The shape's description and lowered module roots: the declared answer's two records.
fn records(aeon: &Path, s: &shapes::Shape) -> Result<(Project, ResolvedShape, Vec<ModuleSpec>), String> {
    let p = s.project(aeon)?;
    let resolved = s.resolved(aeon)?;
    let registry = shapes::emp_program(aeon, s).map_err(|e| format!("shape `{}`: module roots: {e}", s.name()))?.registry;
    Ok((p, resolved, registry))
}

/// Measure one shape: its declared answer (no build output read) and its emitted
/// answer (its own build's listing). Every shape is measured before any is judged, so
/// a run names every disagreeing shape rather than the first.
fn measure(aeon: &Path, s: &shapes::Shape) -> Result<Row, String> {
    let label = s.name();
    let (p, resolved, registry) = records(aeon, s)?;
    let declared = declares_island(&p, &resolved, &registry).map_err(|e| e.to_string())?;
    let built = shapes::build(aeon, s).map_err(|e| format!("shape `{label}`: build failed, so nothing was measured: {e}"))?;
    Ok(Row {
        label,
        blob: blob_label(&p).to_string(),
        declared,
        emitted: listing_carries_island(&built.listing, blob_label(&p)),
        symbols: built.listing.len(),
    })
}

/// THE GATE.
#[test]
fn every_shipped_shape_emits_the_island_label_exactly_when_it_declares_one() {
    // Strict mode turns a reference it cannot find into a panic naming the path, so
    // this cannot report green without having built anything.
    let Some(aeon) = reference_tree_for_shape(&shapes::Shape::new("sonic4", "release")) else { return };
    let shipped = shapes::shipped(&aeon).expect("shipped shapes");
    assert!(
        !shipped.is_empty(),
        "shapes::shipped enumerated nothing, a set diff over an empty set is green \
         for the one reason that proves nothing"
    );

    let mut rows: Vec<Row> = Vec::new();
    let mut unmeasured: Vec<String> = Vec::new();
    for s in &shipped {
        match measure(&aeon, s) {
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
        shipped.len(),
        unmeasured.join("\n  "),
    );

    // BOTH DIRECTIONS NEED A POPULATION.
    let carrying = rows.iter().filter(|r| r.declared).count();
    let bare = rows.len() - carrying;
    assert!(
        carrying > 0,
        "no shipped shape declares the error_handler island, so the declares-it-must-\
         emit-it direction is enforced over an empty set."
    );
    assert!(
        bare > 0,
        "every shipped shape declares the error_handler island, so the declares-none-\
         must-emit-none direction is enforced over an empty set. The `lean` shape is \
         the shape that gives that direction a subject."
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
/// in a COPY of the engine tree. The island is still placed and the shape still declares
/// it; the only difference is that the name the debugger's consumers look for is no
/// longer the name the source defines.
///
/// A demo shape carries the witness: it is engine-only, so the doctoring reaches it
/// through the same shared module every game's build reads, and it is sound-OFF.
#[test]
fn renaming_the_blob_label_in_the_engine_tree_makes_the_gate_red() {
    let shape = shapes::Shape::new("demo", "debug");
    let Some(aeon) = reference_tree_for_shape(&shape) else { return };
    let (p, resolved, registry) = records(&aeon, &shape).expect("demo debug records");
    assert!(
        declares_island(&p, &resolved, &registry).expect("the demo debug shape is coherent"),
        "control: this witness needs a shape that DECLARES the island"
    );
    let label = blob_label(&p).to_string();

    // The control half, on the undoctored tree.
    let clean = shapes::build(&aeon, &shape).unwrap_or_else(|e| panic!("undoctored build: {e}"));
    assert!(
        listing_carries_island(&clean.listing, &label),
        "control: the shape declares the island, so its own build defines the blob label"
    );

    let src = std::fs::read_to_string(aeon.join(ISLAND_SOURCE_REL))
        .unwrap_or_else(|e| panic!("read {ISLAND_SOURCE_REL}: {e}"));
    let renamed = format!("{label}Renamed");
    let doctored_src = src.replace(&label, &renamed);
    assert_ne!(doctored_src, src, "the label must occur in {ISLAND_SOURCE_REL}, or this witness doctored nothing");
    let shadow = shadow_aeon_tree(&aeon, &[(ISLAND_SOURCE_REL, &doctored_src)])
        .unwrap_or_else(|e| panic!("shadow tree: {e}"));

    // The game-as-data driver checks membership at build time (the appendix step's
    // blob-end contract runs with `expect_island` from this same declaration), so the
    // doctored build is refused BY THE BUILD, naming the label and the silent runtime
    // symptom; the profile-driven driver returned the image before that check, which is
    // why this witness used to measure a green build.
    let e = measure(shadow.root(), &shape).err().unwrap_or_else(|| {
        panic!("the doctored tree BUILT: the renamed blob label reached no gate")
    });
    assert!(e.contains("demo debug"), "the refusal must name the shape; got: {e}");
    assert!(e.contains("MEMBERSHIP") && e.contains(&label), "the refusal must name the island membership and the label; got: {e}");
    assert!(
        e.contains("<unknown>"),
        "the declares-but-does-not-emit direction must name the silent runtime symptom; got: {e}"
    );

    // And the set diff names the same shape from the two answers, so the gate stays the
    // one place that reads both directions.
    let faults = membership_faults(&[Row { label: shape.name(), blob: label.clone(), declared: true, emitted: false, symbols: clean.listing.len() }]);
    assert_eq!(faults.len(), 1, "the doctored shape must be the one fault: {faults:?}");
    assert!(faults[0].contains("demo debug") && faults[0].contains("<unknown>"), "got: {}", faults[0]);
}

/// RED-FIRST, THE DECLARED SIDE, and the other direction of the diff: a shape whose
/// declaration says it carries no island, judged against a build that does, is named.
#[test]
fn a_shape_that_declares_no_island_but_emits_one_is_named() {
    let Some(aeon) = reference_tree_for_shape(&shapes::Shape::new("sonic4", "release")) else { return };
    let shipped = shapes::shipped(&aeon).expect("shipped shapes");
    let shape = shipped
        .iter()
        .find(|s| {
            records(&aeon, s)
                .ok()
                .and_then(|(p, r, reg)| declares_island(&p, &r, &reg).ok())
                .unwrap_or(false)
        })
        .expect("a shipped shape declares the island");
    let p = shape.project(&aeon).expect("project");
    let built = shapes::build(&aeon, shape).unwrap_or_else(|e| panic!("shape `{}`: {e}", shape.name()));

    let rows = vec![Row {
        label: shape.name(),
        blob: blob_label(&p).to_string(),
        declared: false,
        emitted: listing_carries_island(&built.listing, blob_label(&p)),
        symbols: built.listing.len(),
    }];
    let faults = membership_faults(&rows);
    assert_eq!(faults.len(), 1, "the mis-declared shape must be the one fault: {faults:?}");
    assert!(
        faults[0].contains("lost the split"),
        "the declares-none-but-emits direction is a different fault from the other and must read as one; got: {}",
        faults[0]
    );
}

/// The declared answer is a RECONCILIATION of two records, and this proves it is live
/// in both of its failure modes. Neither probe touches the tree: they doctor a shipped
/// shape's derived module list, or its features, in memory.
#[test]
fn the_declared_answer_refuses_a_profile_whose_two_records_disagree() {
    let sonic4 = shapes::Shape::new("sonic4", "release");
    let Some(aeon) = reference_tree_for_shape(&sonic4) else { return };
    let (p, resolved, derived) = records(&aeon, &sonic4).expect("sonic4 release records");

    // Features keep the crash-report axis but the registry loses the island row.
    assert!(resolved.features.crash_report, "control: this shape's axis says it carries the island");
    let mut half_removed = derived.clone();
    half_removed.retain(|m| m.module_id != p.engine.error_handler);
    let e = declares_island(&p, &resolved, &half_removed).expect_err("axis says island, registry places neither handler").to_string();
    assert!(
        e.contains("neither"),
        "removing one arm leaves the split with NEITHER arm, which is the first thing that stops making sense; got: {e}"
    );

    // Both arms placed: the split is gone the other way. The control first proves the
    // island IS in this shape's registry.
    let mut both_arms = derived.clone();
    assert!(both_arms.iter().any(|m| m.module_id == p.engine.error_handler), "control: the island is in this shape's registry");
    both_arms.push(ModuleSpec { module_id: p.engine.release_fault.clone(), section: "release_fault".to_string() });
    let e = declares_island(&p, &resolved, &both_arms).expect_err("a registry placing both fault handlers has lost the split").to_string();
    assert!(e.contains("both"), "must say which way the split broke; got: {e}");

    // The split intact but the axis flipped: the two records disagree, the arm that
    // catches a registry re-gate that forgot the shape's features.
    let lean = shapes::Shape::new("sonic4", "lean");
    let (lp, mut lean_resolved, lean_registry) = records(&aeon, &lean).expect("lean records");
    assert!(
        !declares_island(&lp, &lean_resolved, &lean_registry).expect("lean is coherent"),
        "control: the lean shape declares no island"
    );
    lean_resolved.features.crash_report = true;
    let e = declares_island(&lp, &lean_resolved, &lean_registry)
        .expect_err("axis says island, registry places the lean handler")
        .to_string();
    assert!(
        e.contains("`debug || crash_report` is true but the gate drops"),
        "must name the disagreement rather than picking a winner; got: {e}"
    );
}
