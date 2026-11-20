//! Flip Stage 2 · S1.2 — THE FROZEN-TABLE PLACEMENT GATE (off-canonical).
//!
//! The committed listing tables (`golden/offcanonical_sizes/*.txt`) are FREEZE RECORDS:
//! every off-canonical section's ROM base, as the frozen golden carries it. Since G0b
//! Task 11 the layout they are held to is the game-as-data driver's (`sigil build
//! --project <tree>/games/<g> --shape <row>`, through `sigil_harness::shapes`), which
//! places by its descriptions' declared alignment walk and reads no table. This gate
//! proves the PLACEMENT half on Config-B (sonic4, sound-off): every frozen-labeled
//! section resolves to its exact frozen address, and each table re-derives exactly.
//!
//! (The profile-driven driver's `t24` gate, `config_b_doctored_size_table_moves_no_bytes`,
//! proved ITS frozen table was not a placement input; the game-as-data driver has no such
//! input to doctor, so that gate retired with the driver.)
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_offcanonical_placement
//! ```
use sigil_harness::repin::{boundary_table_of, parse_size_table};
use sigil_harness::shapes;
use sigil_harness::test_support::reference_tree_for_shape;
use std::collections::{BTreeMap, HashSet};

// The builds share one process-wide capture of the sound seams' output; serialize.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The committed freeze record `golden/offcanonical_sizes/<stem>.txt`, label → address.
fn committed_table(stem: &str) -> BTreeMap<String, u32> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../sigil-harness/golden/offcanonical_sizes/{stem}.txt"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    parse_size_table(&text).into_iter().collect()
}

/// The table measured off the shape's own resolved layout for the committed table's label
/// set (`repin::boundary_table_of`, the measurement `derive_offcanon` writes the tables
/// with): each label's ROM address, the `*_End` markers synthesized from section geometry.
fn measured_table(aeon: &std::path::Path, shape: &shapes::Shape, want: &BTreeMap<String, u32>) -> BTreeMap<String, u32> {
    let labels: HashSet<String> = want.keys().cloned().collect();
    let resolved = shapes::resolved_layout(aeon, shape).unwrap_or_else(|e| panic!("{}: {e}", shape.name()));
    boundary_table_of(&resolved, &labels).unwrap_or_else(|e| panic!("{}: {e}", shape.name()))
}

/// Config-B: every frozen-labeled section lands at its committed config_b address
/// (TestPlayer/OJZ/error-handler/EndOfRom), not sonic4's — the empty mismatch set.
#[test]
fn config_b_frozen_placement_exact() {
    let shape = shapes::Shape::new("sonic4", "config-b");
    let Some(aeon) = reference_tree_for_shape(&shape) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let committed = committed_table("config_b");
    let measured = measured_table(&aeon, &shape, &committed);
    let mismatches: Vec<(&String, u32, Option<&u32>)> =
        committed.iter().filter(|(k, v)| measured.get(*k) != Some(v)).map(|(k, v)| (k, *v, measured.get(k))).collect();
    assert!(
        mismatches.is_empty(),
        "config_b misplaced {} declared section(s) against the freeze record; first (label, frozen, placed): {:?}",
        mismatches.len(),
        mismatches.first()
    );
}

// ── P4a — THE LMA-CORRECT SIZE DERIVATION ──
//
// Each boundary label's ROM address is read off sigil's OWN resolved layout
// (`section.lma + label.offset`), the section-END markers synthesized from section
// geometry. This reproduces the committed table EXACTLY for every off-canonical target:
// sigil's resolve is the size authority (no asl listing is parsed). Since G0b Task 11
// the layout is the game-as-data driver's and the tables are freeze records it is held to.

/// For each off-canonical target: the measured table equals the committed one (same
/// label set, same addresses), including the phased z80 idle (ROM LMA `$3d8`, not VMA
/// `$0`) and the synthesized `*_End` markers.
fn rederives_native(shape: shapes::Shape, stem: &str) {
    let Some(aeon) = reference_tree_for_shape(&shape) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let committed = committed_table(stem);
    let derived = measured_table(&aeon, &shape, &committed);
    assert_eq!(
        derived,
        committed,
        "{}: LMA-native derivation diverged from the committed table (re-derive the \
         offcanonical_sizes/*.txt with `derive_offcanon`?)",
        shape.name()
    );
}

#[test]
fn demo_size_table_rederives_native() {
    rederives_native(shapes::Shape::new("demo", "release"), "demo");
}
#[test]
fn demo_debug_size_table_rederives_native() {
    rederives_native(shapes::Shape::new("demo", "debug"), "demo_debug");
}
#[test]
fn config_a_size_table_rederives_native() {
    rederives_native(shapes::Shape::new("sonic4", "config-a"), "config_a");
}
#[test]
fn config_b_size_table_rederives_native() {
    rederives_native(shapes::Shape::new("sonic4", "config-b"), "config_b");
}
/// LEAN (crash-report OFF) — the 7th frozen table. It is the only committed table that
/// carries a `ReleaseFault` row and no `BusError`, so this is where a regression that
/// quietly re-attached the error_handler island to a CRASH_REPORT=0 build would surface
/// as a label-set divergence rather than a byte diff.
#[test]
fn lean_size_table_rederives_native() {
    rederives_native(shapes::Shape::new("sonic4", "lean"), "lean");
}

/// Wave-B B-0b — the RAM-packing invariant guard. RAM is placed by AS `phase`/
/// `dephase` (`engine/ram.asm` at `$FFFF0000`/`$FFFF8000`) plus `phase Engine_RAM_End`
/// (game RAM chains onto the engine block) — the RAM analog of B-0's contiguous
/// packing, executed natively by sigil's AS frontend. This gate asserts the three
/// structural properties that keep a RAM-growing parcel (entity_window #1, tile_cache
/// #2) safe, in BOTH shapes:
///   (a) every RAM section (`vma_origin >= $FFFF0000`) is EVEN-based — the 68k
///       address-error guard;
///   (b) NO ROM section lands in RAM (`is_rom_section` partition holds) — ROM bases
///       are independent of RAM sizes, so RAM growth never perturbs the ROM layout;
///   (c) upper RAM PACKS CONTIGUOUSLY — sections at `vma >= $FFFF8000`, sorted, each
///       successor base == predecessor end (game RAM abuts `Engine_RAM_End`). A gap
///       or overlap means the packing broke.
/// See `docs/superpowers/notes/2026-08-01-waveb-b0b-ram-packing.md` for the growth
/// probe that exercised these under a live +2 RAM growth.
fn ram_packing_invariants(debug: bool) {
    let shape = shapes::Shape::new("sonic4", if debug { "debug" } else { "release" });
    let Some(aeon) = reference_tree_for_shape(&shape) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let secs = shapes::resolved_layout(&aeon, &shape).unwrap_or_else(|e| {
        panic!("resolve canonical sections ({}): {e}", if debug { "debug" } else { "plain" })
    });

    let mut ram: Vec<&sigil_ir::Section> =
        secs.iter().filter(|s| s.vma_origin() >= 0xFFFF_0000).collect();
    ram.sort_by_key(|s| s.vma_origin());

    // (a) even-based, and there IS a RAM region (guards against a resolve that lost it).
    assert!(!ram.is_empty(), "no RAM sections resolved, the phase blocks vanished");
    for s in &ram {
        assert_eq!(
            s.vma_origin() % 2,
            0,
            "RAM section `{}` base {:#x} is ODD, a 68k word/long access there address-errors",
            s.name,
            s.vma_origin()
        );
    }

    // (b) partition: no ROM section carries a high (RAM) lma; RAM sizes cannot move ROM.
    let rom_in_ram = secs
        .iter()
        .filter(|s| s.vma_origin() < 0xFFFF_0000 && s.lma >= 0xFFFF_0000)
        .count();
    assert_eq!(
        rom_in_ram, 0,
        "{rom_in_ram} ROM section(s) placed at a RAM lma, the ROM/RAM partition broke; \
         RAM growth could perturb ROM bases"
    );

    // (c) upper RAM (vma >= $FFFF8000) packs contiguously: successor base == predecessor end.
    let upper: Vec<&sigil_ir::Section> =
        ram.iter().copied().filter(|s| s.vma_origin() >= 0xFFFF_8000).collect();
    for w in upper.windows(2) {
        let end = w[0].vma_origin() + w[0].reserved_span;
        assert_eq!(
            w[1].vma_origin(),
            end,
            "upper-RAM packing broke: `{}` ends {:#x} but `{}` begins {:#x} \
             (game RAM must abut Engine_RAM_End with no gap/overlap)",
            w[0].name,
            end,
            w[1].name,
            w[1].vma_origin()
        );
    }
}

#[test]
fn ram_packing_invariants_plain() {
    ram_packing_invariants(false);
}
#[test]
fn ram_packing_invariants_debug() {
    ram_packing_invariants(true);
}
