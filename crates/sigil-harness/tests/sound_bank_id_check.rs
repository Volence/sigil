//! `[sound.bank-id-vs-placement]` against the reference tree: the sites the check
//! finds are the ones the sources declare, the shipped layouts pass with every site
//! measured, a layout that places a bank off its baked window is refused naming both
//! ids, and a moved map anchor fails the whole build with this diagnostic.
//!
//! Every expectation is derived from aeon source or from the layout under test:
//! the blob site counts from the resident modules' uses of each bank-id const, the
//! DAC site split from `dac_sample_tab.emp`'s descriptor rows and `dac_samples.emp`'s
//! sections, the ids from `seam2::bank_id_of` over placed LMAs.

use sigil_harness::map_placement::load_placement_map;
use sigil_build::describe::SoundDesc;
use sigil_harness::seam2::bank_id_of;
use sigil_harness::shapes::{self, Shape};
use sigil_harness::sound_bank_ids::{blob_bank_id_consts, validate_sound_bank_ids};
use sigil_harness::test_support::{read_dac_declarations, reference_tree_for_shape};
use sigil_ir::Section;
use std::path::Path;

/// The five resident Z80 modules the blob links (seam-1's file order).
const RESIDENT: &[&str] = &[
    "engine/sound/z80_sound_driver.emp",
    "engine/sound/sound_sequencer.emp",
    "engine/sound/sound_sfx.emp",
    "engine/sound/sound_fm.emp",
    "engine/sound/sound_psg.emp",
];

/// How many instructions in the resident modules take `name` as an operand: code
/// lines (comment stripped) that mention it as a whole token.
fn operand_uses(aeon: &Path, name: &str) -> usize {
    let mut n = 0;
    for rel in RESIDENT {
        let src = std::fs::read_to_string(aeon.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        for line in src.lines() {
            let code = line.split("//").next().unwrap_or("");
            let code = code.trim_start();
            // An instruction line; a `const`/`use`/`ensure` naming it is not an operand.
            if code.starts_with("ld") {
                n += code
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .filter(|t| *t == name)
                    .count();
            }
        }
    }
    n
}

/// For each `dc.b SND_<X>_BANK` descriptor row of `dac_sample_tab.emp`, in order:
/// true when `<X>`'s bank label is declared in `dac_blip_bank`, false when in
/// `dac_shared_bank`.
fn dac_rows_in_blip_bank(aeon: &Path) -> Vec<bool> {
    let decl = read_dac_declarations(&aeon.join("games/sonic4/data/sound"));
    let tab = std::fs::read_to_string(aeon.join("engine/sound/dac_sample_tab.emp")).expect("read dac_sample_tab.emp");
    let section_of = |label: &str| -> String {
        decl.sections
            .iter()
            .find(|(_, lines)| lines.iter().any(|l| l.label == label))
            .map(|(s, _)| s.clone())
            .unwrap_or_else(|| panic!("dac_samples.emp places no data line `{label}`"))
    };
    let mut rows = Vec::new();
    for line in tab.lines() {
        let code = line.split("//").next().unwrap_or("").trim();
        let Some(rest) = code.strip_prefix("dc.b") else { continue };
        let operand = rest.trim();
        let Some(base) = operand.strip_prefix("SND_").and_then(|s| s.strip_suffix("_BANK")) else { continue };
        let label = decl.equs[base].bank_of.clone().unwrap_or_else(|| panic!("SND_{base}_BANK names no label"));
        match section_of(&label).as_str() {
            "dac_blip_bank" => rows.push(true),
            "dac_shared_bank" => rows.push(false),
            other => panic!("`{label}` sits in section `{other}`, not a DAC bank"),
        }
    }
    assert!(!rows.is_empty(), "dac_sample_tab.emp declares no `dc.b SND_*_BANK` row");
    rows
}

fn sonic4(debug: bool) -> Shape {
    Shape::new("sonic4", if debug { "debug" } else { "release" })
}

/// The engine's sound facts (`engine.toml [sound]`), as the build reads them.
fn sound_desc(aeon: &Path) -> SoundDesc {
    sonic4(false).project(aeon).expect("project").engine.sound
}

#[test]
fn sites_are_the_ones_the_sources_declare() {
    let Some(aeon) = reference_tree_for_shape(&sonic4(false)) else { return };
    let sound = sound_desc(&aeon);
    let consts = blob_bank_id_consts(&sound);
    assert!(!consts.is_empty(), "engine.toml [[sound.blob_bank_ids]] names no const, the check would measure nothing");
    for debug in [false, true] {
        let sites = sigil_harness::seam1::blob_bank_id_sites(&aeon, &consts, debug).expect("blob sites");
        let names: Vec<String> = sites.by_const.iter().map(|(n, _)| n.clone()).collect();
        assert_eq!(names, consts);
        for (name, offsets) in &sites.by_const {
            assert_eq!(
                offsets.len(),
                operand_uses(&aeon, name),
                "{name} ({}): sites found {offsets:x?}",
                if debug { "debug" } else { "plain" }
            );
        }
    }
    let rows = dac_rows_in_blip_bank(&aeon);
    let dac = sigil_harness::seam2::dac_head_bank_id_sites(&aeon).expect("dac sites");
    let mut all: Vec<(u32, bool)> =
        dac.blip.iter().map(|&o| (o, true)).chain(dac.shared.iter().map(|&o| (o, false))).collect();
    all.sort();
    let got: Vec<bool> = all.iter().map(|&(_, b)| b).collect();
    assert_eq!(got, rows, "DAC ds_bank sites {all:x?} against the descriptor rows' banks");
}

/// The shape's resolved layout and its linked image.
fn resolved_and_linked(aeon: &Path, profile: &Shape) -> (Vec<Section>, sigil_link::LinkedImage) {
    let resolved = shapes::resolved_layout(aeon, profile).expect("resolve");
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
    (resolved, linked)
}

/// Sites times the bank labels each is checked against: SND_ENGINE_TABLE_BANK against
/// one label, SFX_BLOB_BANK against two, each DAC descriptor against one.
fn expected_comparisons(aeon: &Path) -> usize {
    let engine = operand_uses(aeon, "SND_ENGINE_TABLE_BANK");
    let sfx = operand_uses(aeon, "SFX_BLOB_BANK");
    engine + 2 * sfx + dac_rows_in_blip_bank(aeon).len()
}

#[test]
fn the_shipped_layouts_pass_with_every_site_measured() {
    let Some(aeon) = reference_tree_for_shape(&sonic4(false)) else { return };
    let sound = sound_desc(&aeon);
    for debug in [false, true] {
        let profile = sonic4(debug);
        let (resolved, linked) = resolved_and_linked(&aeon, &profile);
        let n = validate_sound_bank_ids(&aeon, &sound, &resolved, &linked, profile.features(&aeon).sound, debug)
            .unwrap_or_else(|e| panic!("sonic4 debug={debug}: {e}"));
        assert_eq!(n, expected_comparisons(&aeon), "sonic4 debug={debug}");
    }
    // A sound-off shape has nothing to check, and its layout confirms it.
    let demo = Shape::new("demo", "release");
    assert!(!demo.features(&aeon).sound);
    let (resolved, linked) = resolved_and_linked(&aeon, &demo);
    assert_eq!(validate_sound_bank_ids(&aeon, &sound, &resolved, &linked, false, false), Ok(0));
    // The same sound-on layout declared sound-off is refused, not skipped.
    let profile = sonic4(false);
    let (resolved, linked) = resolved_and_linked(&aeon, &profile);
    let e = validate_sound_bank_ids(&aeon, &sound, &resolved, &linked, false, false).unwrap_err();
    assert!(e.contains("declares sound off"), "{e}");
}

/// The LMA of `label` in `resolved`, and the index of its section.
fn find(resolved: &[Section], label: &str) -> (usize, u32) {
    resolved
        .iter()
        .enumerate()
        .find_map(|(i, s)| s.labels.iter().find(|l| l.name == label).map(|l| (i, s.lma + l.offset)))
        .unwrap_or_else(|| panic!("`{label}` not placed"))
}

#[test]
fn a_bank_placed_off_its_baked_window_is_refused_naming_both_ids() {
    let Some(aeon) = reference_tree_for_shape(&sonic4(false)) else { return };
    let sound = sound_desc(&aeon);
    let profile = sonic4(false);
    let (mut resolved, linked) = resolved_and_linked(&aeon, &profile);
    // Move the DAC bank section by whole windows after the image was linked: the baked
    // bytes stay, the placement the check reads moves.
    const MOVE: u32 = 0x10_0000;
    let (sec, blip) = find(&resolved, "Dac_Temp_Blip");
    let (_, shared) = find(&resolved, "Dac_SharedBank_Start");
    resolved[sec].lma += MOVE;
    let e = validate_sound_bank_ids(&aeon, &sound, &resolved, &linked, true, false).unwrap_err();
    assert!(e.starts_with("[sound.bank-id-vs-placement]"), "{e}");
    let rows = dac_rows_in_blip_bank(&aeon);
    for (bank, label, n) in [
        (blip, "Dac_Temp_Blip", rows.iter().filter(|b| **b).count()),
        (shared, "Dac_SharedBank_Start", rows.iter().filter(|b| !**b).count()),
    ] {
        let line = format!(
            "= ${:02X}, but `{label}` was placed at {:#x}, bank ${:02X}",
            bank_id_of(bank),
            bank + MOVE,
            bank_id_of(bank + MOVE)
        );
        assert_eq!(e.matches(&line).count(), n, "`{line}` in:\n{e}");
    }
    // Nothing else is reported: the blob's sites still match their unmoved banks.
    assert_eq!(e.lines().filter(|l| l.starts_with("  ")).count(), rows.len(), "{e}");
}

/// The seams' sound emit, run under a moved `dac_banks` anchor while the build places
/// the banks where the map says: every other seam is the harness's own.
struct EmitUnderMovedDac {
    moved: sigil_harness::map_placement::AnchorOverlay,
}

impl sigil_build::driver::Seams for EmitUnderMovedDac {
    fn emit_sound(
        &self,
        checkout: &Path,
        generated: &Path,
        _overlay: Option<&sigil_harness::map_placement::AnchorOverlay>,
        debug: bool,
    ) -> Result<Box<dyn std::any::Any>, String> {
        sigil_harness::driver_seams::HarnessSeams.emit_sound(checkout, generated, Some(&self.moved), debug)
    }
    fn validate_sound(
        &self,
        checkout: &Path,
        sound: &SoundDesc,
        overlay: Option<&sigil_harness::map_placement::AnchorOverlay>,
        resolved: &[Section],
        linked: &sigil_link::LinkedImage,
        sound_on: bool,
        debug: bool,
    ) -> Result<(), String> {
        sigil_harness::driver_seams::HarnessSeams.validate_sound(checkout, sound, overlay, resolved, linked, sound_on, debug)
    }
    fn side_tables(&self, i: sigil_build::driver::SideInputs<'_>) -> Result<sigil_link::export::SideTables, String> {
        sigil_harness::driver_seams::HarnessSeams.side_tables(i)
    }
}

/// The whole build refuses a ROM whose baked DAC bank ids were folded from a different
/// `dac_banks` anchor than the one the banks were placed at, naming the baked byte.
///
/// G0b Task 11: the profile-driven driver placed the DAC section by its frozen row, so
/// moving the map's `dac_banks` anchor alone split the emit from the placement. The
/// game-as-data driver places the bank AT the anchor (a moved anchor builds green with
/// the ids agreeing), so the split is planted at the seam instead: the sound emit runs
/// under an anchor overlay that moves `dac_banks` by one window, the placement reads
/// the unmoved map.
#[test]
fn a_sound_emit_under_a_moved_dac_anchor_fails_the_build_with_this_diagnostic() {
    use sigil_build::driver::{build, BuildOptions};
    let profile = sonic4(false);
    let Some(aeon) = reference_tree_for_shape(&profile) else { return };
    let map_rel = "games/sonic4/map.toml";
    let src = std::fs::read_to_string(aeon.join(map_rel)).expect("read map.toml");
    let pmap = load_placement_map(&src).expect("map");
    let at = |name: &str| pmap.anchors_for(true).find(|a| a.name == name).unwrap_or_else(|| panic!("{name}")).at;
    let dac = at("dac_banks");
    // One window down: inside the data region, below the shared bank and the head bank.
    let moved = dac - 0x8000;
    let ov = sigil_harness::map_placement::parse_anchor_overlay(
        &format!("[[anchor]]\nname = \"dac_banks\"\nat = {moved:#X}\nwhen = \"sound_on\"\n"),
        "test: dac_banks one window down",
    )
    .expect("overlay");
    let planted = EmitUnderMovedDac { moved: ov };
    let env = |_: &str| None;
    let opts = |seams: &'static dyn sigil_build::driver::Seams| BuildOptions {
        extra_entries: Vec::new(),
        anchor_overlay: None,
        layout_pass: Default::default(),
        seams,
        env: &env,
        generated: None,
    };
    // Control: the same build with the harness's own seams is green.
    build(&profile.game_dir(&aeon), "release", &opts(&sigil_harness::driver_seams::HarnessSeams))
        .unwrap_or_else(|e| panic!("control build: {e}"));
    let planted: &'static EmitUnderMovedDac = Box::leak(Box::new(planted));
    let opts = opts(planted);
    let e = match build(&profile.game_dir(&aeon), "release", &opts) {
        Ok(_) => panic!("a sound emit folded under dac_banks = {moved:#x} built green against the placed {dac:#x}"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("[sound.bank-id-vs-placement]"), "{e}");
    assert!(
        e.contains(&format!("bakes ds_bank (dac_blip_bank) = ${:02X}, but `Dac_Temp_Blip`", bank_id_of(moved))),
        "{e}"
    );
}

