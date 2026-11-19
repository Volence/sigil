//! The game-as-data driver end to end (G0b Task 5, fix round 1): `sigil_build::driver::build`
//! builds every shipped shape of every game the reference tree describes, and each ROM is what
//! the descriptions and the driver's own guards say it must be.
//!
//! "Shipped" is read from the descriptions: every `[shapes.*]` row that is not a `fixture`.
//! Every expectation is derived from source (the engine and game descriptions, the map, the
//! build's own listing), never from a pinned measurement. Byte identity with the old driver is
//! NOT asserted: it is not reached yet (QUEUE G0B-TASK5-WALK-IDENTITY).
//!
//! Per shape:
//!   * the build succeeds;
//!   * the listing's `EndOfRom` is the assembled image length the driver reports;
//!   * a shape whose features carry the crash report (`debug || crash_report`) places
//!     `engine.toml`'s `error_handler` module, its blob ends exactly at `EndOfRom`
//!     (`error_handler_blob` + `error_handler_blob_len`), and the file carries a deb2 appendix
//!     there (magic, size inside `[appendix_floor, 0x10000]`); any other shape places the
//!     `release_fault` module instead and its file is the assembled image, nothing appended;
//!   * the Sega header's ROM-end field is the last byte's address and its checksum is the one
//!     the header fold computes over the file;
//!   * every `[[anchor]]` in force puts the head label of the row it `holds` at its `at`.
//!
//! The build writes the generated sound artifacts into the reference tree, as the old
//! driver's tests do (`seam2::require_reference_tree` guards that write).

use std::path::{Path, PathBuf};

use sigil_build::describe::{load_engine, load_game, resolve_shape, EngineDesc, GameDesc};
use sigil_build::driver::{self, BuildOptions};
use sigil_harness::driver_seams::HarnessSeams;
use sigil_harness::test_support;

fn shipped_shapes(game: &GameDesc) -> Vec<String> {
    game.shapes
        .keys()
        .filter(|name| !resolve_shape(game, name).unwrap_or_else(|e| panic!("shape `{name}`: {e}")).fixture)
        .cloned()
        .collect()
}

fn label(listing: &[sigil_link::ListingSymbol], name: &str) -> Option<u32> {
    listing.iter().find(|l| l.name == name && !l.is_equate).map(|l| l.value)
}

fn check_shape(aeon: &Path, game_dir: &Path, game: &GameDesc, engine: &EngineDesc, shape: &str) -> Vec<String> {
    let mut faults = Vec::new();
    let mut fault = |m: String| faults.push(format!("{} `{shape}`: {m}", game.id));
    let env = |k: &str| std::env::var_os(k);
    let seams = HarnessSeams;
    let opts = BuildOptions {
        extra_entries: Vec::new(),
        anchor_overlay: None,
        layout_pass: sigil_frontend_emp::export_side::LayoutPass::Off,
        seams: &seams,
        env: &env,
    };
    let out = match driver::build(game_dir, shape, &opts) {
        Ok(o) => o,
        Err(e) => {
            fault(format!("the build failed: {e}"));
            return faults;
        }
    };
    assert_eq!(
        std::fs::canonicalize(&out.checkout).unwrap(),
        std::fs::canonicalize(aeon).unwrap(),
        "the driver read a checkout other than the reference tree"
    );
    let features = resolve_shape(game, shape).unwrap().features;
    let eor = out.assembled_len;
    match label(&out.listing, "EndOfRom") {
        Some(v) if v as usize == eor => {}
        other => fault(format!("listing EndOfRom is {other:x?}, the driver reports an assembled length of {eor:#x}")),
    }
    let places = |m: &str| out.registry.iter().any(|s| s.module_id == m);
    let island = features.debug || features.crash_report;
    if places(&engine.error_handler) != island || places(&engine.release_fault) == island {
        fault(format!(
            "features say island = {island}, but the registry places `{}` = {} and `{}` = {}",
            engine.error_handler,
            places(&engine.error_handler),
            engine.release_fault,
            places(&engine.release_fault)
        ));
    }
    let rom = &out.rom;
    if island {
        match label(&out.listing, &engine.error_handler_blob) {
            Some(b) if (b + engine.error_handler_blob_len) as usize == eor => {}
            other => fault(format!(
                "`{}` at {other:x?} + {:#x} is not EndOfRom {eor:#x}",
                engine.error_handler_blob, engine.error_handler_blob_len
            )),
        }
        if rom.get(eor..eor + 2) != Some(&sigil_build::appendix::DEB2_MAGIC[..]) {
            fault(format!("no deb2 magic at EndOfRom {eor:#x}"));
        }
        let app = rom.len().saturating_sub(eor);
        if !(game.appendix_floor as usize..=0x10000).contains(&app) {
            fault(format!("deb2 appendix is {app:#x} bytes, outside [{:#x}, 0x10000]", game.appendix_floor));
        }
    } else if rom.len() != eor {
        fault(format!("no island, yet the file is {:#x} bytes against an image of {eor:#x}", rom.len()));
    }
    if rom.len() >= 0x200 {
        let end = u32::from_be_bytes(rom[0x1A4..0x1A8].try_into().unwrap());
        if end as usize != rom.len() - 1 {
            fault(format!("header ROM end {end:#x}, the last byte is at {:#x}", rom.len() - 1));
        }
        let mut folded = rom.clone();
        sigil_link::apply_header_checksum(&mut folded);
        if folded[0x18E..0x190] != rom[0x18E..0x190] {
            fault(format!("header checksum {:02x?} is not the fold's {:02x?}", &rom[0x18E..0x190], &folded[0x18E..0x190]));
        }
    } else {
        fault(format!("the file is {} bytes, shorter than a Sega header", rom.len()));
    }
    let map_src = std::fs::read_to_string(game_dir.join(&game.map)).unwrap();
    let pmap = sigil_build::map_placement::load_placement_map(&map_src).unwrap();
    let mut anchors = 0;
    for a in pmap.anchors_for(features.sound) {
        let holds = a.holds.clone().unwrap_or_default();
        if sigil_build::map_placement::section_row(&holds).is_some() {
            continue;
        }
        anchors += 1;
        match label(&out.listing, &holds) {
            Some(v) if v == a.vma.unwrap_or(a.at) || v == a.at => {}
            other => fault(format!("anchor `{}` holds `{holds}` at {:#x}, the listing puts it at {other:x?}", a.name, a.at)),
        }
    }
    if anchors == 0 {
        fault("UNMEASURABLE: no label-row anchor is in force, so the placement check had no subject".into());
    }
    faults
}

fn run_game(id: &str) {
    let rel = format!("games/{id}/game.toml");
    let Some(aeon) = test_support::reference_tree(&["engine/engine.toml", &rel]) else {
        return;
    };
    let engine = load_engine(&aeon.join("engine/engine.toml")).unwrap_or_else(|e| panic!("{e}"));
    let game_path: PathBuf = aeon.join(&rel);
    let game = load_game(&game_path).unwrap_or_else(|e| panic!("{e}"));
    let game_dir = game_path.parent().unwrap().to_path_buf();
    let shapes = shipped_shapes(&game);
    assert!(!shapes.is_empty(), "UNMEASURABLE: {rel} declares no shipped (non-fixture) shape");
    let faults: Vec<String> = shapes.iter().flat_map(|s| check_shape(&aeon, &game_dir, &game, &engine, s)).collect();
    assert!(faults.is_empty(), "{} fault(s) over {} shape(s) {shapes:?}:\n  {}", faults.len(), shapes.len(), faults.join("\n  "));
    eprintln!("game_as_data_driver: {id}: {} shipped shape(s) built and checked: {shapes:?}", shapes.len());
}

#[test]
fn every_shipped_sonic4_shape_builds_as_its_descriptions_say() {
    run_game("sonic4");
}

#[test]
fn every_shipped_demo_shape_builds_as_its_descriptions_say() {
    run_game("demo");
}
