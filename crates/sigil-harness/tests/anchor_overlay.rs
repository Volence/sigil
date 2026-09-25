//! The anchor overlay (`--anchor-overlay`) against the reference tree: the sound
//! derivation reads it without sharing a memo with the map as written, a build under
//! it holds each island at its overlay address with every post-link check passing,
//! a build under an overlay nothing can place fails, and no shipping profile carries
//! one.
//!
//! Every overlay here is DERIVED from the tree's own `map.toml` anchors, never typed:
//! the clip case moves `dac_banks` onto the map's `sound_bank` address and
//! `sound_bank` one DAC-to-sound distance above it, which is the collision a one-pass
//! substitution has to get right. Every overlay build runs in a shadow tree, because
//! a build writes its sound artifacts into the tree, and the reference tree's are
//! shared with every canonical test running beside this one.

use sigil_harness::map_placement::{load_placement_map, parse_anchor_overlay, AnchorOverlay};
use sigil_harness::native::{self, GameProfile};
use sigil_harness::seam2::{bank_id_of, sound_bank_id_in, sound_layout_in};
use sigil_harness::test_support::{reference_tree_for_profile, shadow_aeon_tree};
use std::path::Path;

/// The map's `(dac_banks, sound_bank)` anchor addresses.
fn map_bank_anchors(aeon: &Path) -> (u32, u32) {
    let src = std::fs::read_to_string(aeon.join("games/sonic4/map.toml")).expect("read map.toml");
    let pmap = load_placement_map(&src).expect("map");
    let at = |name: &str| pmap.anchors_for(true).find(|a| a.name == name).unwrap_or_else(|| panic!("{name}")).at;
    (at("dac_banks"), at("sound_bank"))
}

/// An overlay moving the two bank anchors to `dac`/`snd`, spelled as map.toml
/// spells those two rows.
fn bank_overlay(dac: u32, snd: u32) -> AnchorOverlay {
    let src = format!(
        "[[anchor]]\nname = \"dac_banks\"\nat = {dac:#x}\nwhen = \"sound_on\"\n\n\
         [[anchor]]\nname = \"sound_bank\"\nat = {snd:#x}\nvma = 0x8000\nwhen = \"sound_on\"\n"
    );
    parse_anchor_overlay(&src, "fixture/anchors.toml").expect("fixture overlay parses")
}

/// The clip case: `dac_banks` onto the map's `sound_bank`, `sound_bank` one
/// DAC-to-sound distance above it.
fn clip_overlay(aeon: &Path) -> (u32, u32, AnchorOverlay) {
    let (dac, snd) = map_bank_anchors(aeon);
    let (to_dac, to_snd) = (snd, snd + (snd - dac));
    (to_dac, to_snd, bank_overlay(to_dac, to_snd))
}

/// Canonical, then overlay, then canonical, in one process: three layouts, the first
/// and third equal, the middle one derived from the overlay's anchors. A memo keyed
/// by the tree alone returns the first layout three times.
#[test]
fn one_process_derives_the_map_and_the_overlay_layouts_apart() {
    let Some(aeon) = reference_tree_for_profile(&native::sonic4_profile(false)) else { return };
    let (to_dac, to_snd, ov) = clip_overlay(&aeon);
    let first = sound_layout_in(&aeon, None).expect("canonical layout");
    let middle = sound_layout_in(&aeon, Some(&ov)).expect("overlay layout");
    let third = sound_layout_in(&aeon, None).expect("canonical layout again");
    assert_eq!(first, third);
    assert_ne!(first, middle);
    assert_eq!((middle.dac_blip_lma, middle.sound_tables_z80_lma), (to_dac, to_snd));
    let (dac, snd) = map_bank_anchors(&aeon);
    assert_eq!((first.dac_blip_lma, first.sound_tables_z80_lma), (dac, snd));
    // The head bank keeps its internal shape: every head sits the same distance above
    // its bank's start.
    assert_eq!(middle.dac_sample_tab_lma - to_snd, first.dac_sample_tab_lma - snd);
    assert_eq!(sound_bank_id_in(&aeon, Some(&ov)).unwrap(), bank_id_of(to_snd));
    assert_eq!(sound_bank_id_in(&aeon, None).unwrap(), bank_id_of(snd));
}

/// A listing label's ROM address.
fn listed_lma(listing: &[sigil_link::ListingSymbol], name: &str) -> u32 {
    let s = listing
        .iter()
        .find(|s| s.name == name && !s.is_equate)
        .unwrap_or_else(|| panic!("`{name}` is not in the listing"));
    s.lma.unwrap_or(s.value)
}

/// Both sonic4 shapes under the clip overlay build, with `Dac_Temp_Blip` at the
/// overlay's `dac_banks` and `SoundTablesZ80_Head` at its `sound_bank`. The build runs
/// every post-link check (`[sound.bank-id-vs-placement]`, `validate_placement`, the
/// sound fold), so a green build is those checks passing over the overlay.
#[test]
fn an_overlay_build_holds_each_island_at_its_overlay_address() {
    let Some(aeon) = reference_tree_for_profile(&native::sonic4_profile(false)) else { return };
    let (to_dac, to_snd, ov) = clip_overlay(&aeon);
    let shadow = shadow_aeon_tree(&aeon, &[]).expect("shadow tree");
    for debug in [false, true] {
        let profile = native::sonic4_profile(debug).with_anchor_overlay(ov.clone());
        let built = native::build_rom_chained_with_listing(shadow.root(), &profile)
            .unwrap_or_else(|e| panic!("the overlay build (debug={debug}) failed:\n{e}"));
        assert_eq!(listed_lma(&built.listing, "Dac_Temp_Blip"), to_dac, "debug={debug}");
        assert_eq!(listed_lma(&built.listing, "SoundTablesZ80_Head"), to_snd, "debug={debug}");
        // Aeon's banked-data co-residency: the MT bank starts inside the sound bank window.
        assert_eq!(
            bank_id_of(listed_lma(&built.listing, "Song_MovingTrucks")),
            bank_id_of(to_snd),
            "debug={debug}"
        );
    }
}

/// An overlay nothing can place: both bank anchors moved into the object bank, which
/// the packed run already fills. The build must fail; a green build here means the
/// overlay was never consulted.
#[test]
fn an_overlay_nothing_can_place_fails_the_build() {
    let profile = native::sonic4_profile(false);
    let Some(aeon) = reference_tree_for_profile(&profile) else { return };
    let src = std::fs::read_to_string(aeon.join("games/sonic4/map.toml")).expect("read map.toml");
    let pmap = load_placement_map(&src).expect("map");
    let object_bank = pmap.anchors.iter().find(|a| a.name == "object_bank").expect("object_bank anchor").at;
    let (dac, snd) = map_bank_anchors(&aeon);
    let (to_dac, to_snd) = (object_bank + 0x1_0000, object_bank + 0x1_0000 + (snd - dac));
    assert!(to_snd < dac, "the fixture overlay must sit below the map's DAC anchor");
    let shadow = shadow_aeon_tree(&aeon, &[]).expect("shadow tree");
    match native::build_rom_chained_with_listing(shadow.root(), &profile.with_anchor_overlay(bank_overlay(to_dac, to_snd))) {
        Ok(_) => panic!("banks moved to {to_dac:#x}/{to_snd:#x}, inside the packed run, built green"),
        // The DAC bank section is placed at the overlay address and collides there.
        Err(e) => assert!(e.contains("overlap") && e.contains(&format!("[{to_dac:#X},")), "{e}"),
    }
}

/// The shipping profiles carry no overlay: without the switch, nothing reads one.
#[test]
fn no_shipping_profile_carries_an_overlay() {
    let profiles: Vec<GameProfile> = vec![
        native::sonic4_profile(false),
        native::sonic4_profile(true),
        native::demo_profile(false),
        native::demo_profile(true),
        native::config_a_profile(),
        native::config_b_profile(),
        native::lean_profile(),
        native::stress_evict_profile(),
        native::stress_art_profile(),
    ];
    for p in profiles {
        assert!(p.anchor_overlay.is_none(), "profile `{}` carries an anchor overlay", p.name);
    }
}

/// A scratch output directory for one binary run, removed on drop.
struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("sigil-anchor-overlay-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("mkdir scratch");
        Scratch(p)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `emit_sound_blob --anchor-overlay` emits the DAC head and the resident blob from the
/// overlay: its files equal the library's emit under the same overlay and differ from
/// the map's.
#[test]
fn the_emit_binary_emits_from_the_overlay() {
    let Some(aeon) = reference_tree_for_profile(&native::sonic4_profile(false)) else { return };
    let (_, _, ov) = clip_overlay(&aeon);
    let scratch = Scratch::new("emit");
    let file = scratch.0.join("anchors.toml");
    std::fs::write(&file, overlay_text(&ov)).expect("write overlay");
    let out = scratch.0.join("generated");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_emit_sound_blob"))
        .arg("--aeon")
        .arg(&aeon)
        .arg("--out-dir")
        .arg(&out)
        .arg("--anchor-overlay")
        .arg(&file)
        .output()
        .expect("run emit_sound_blob");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let read = |name: &str| std::fs::read(out.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let want_head = sigil_harness::seam2::emit_dac_body_and_head_in(&aeon, Some(&ov)).unwrap().head;
    let map_head = sigil_harness::seam2::emit_dac_body_and_head_in(&aeon, None).unwrap().head;
    assert_ne!(want_head, map_head, "the fixture overlay must move a DAC bank id");
    assert_eq!(read("dac_sample_tab.bin"), want_head);
    let want_blob = sigil_harness::seam1::native_blob_checked_in(&aeon, Some(&ov), false, None).unwrap();
    let map_blob = sigil_harness::seam1::native_blob_checked_in(&aeon, None, false, None).unwrap();
    assert_ne!(want_blob, map_blob, "the fixture overlay must move the sound bank id");
    assert_eq!(read("z80_sound_blob.bin"), want_blob);
}

/// The overlay's rows spelled back as a file.
fn overlay_text(ov: &AnchorOverlay) -> String {
    let mut s = String::new();
    for a in &ov.rows {
        s.push_str(&format!("[[anchor]]\nname = \"{}\"\nat = {:#x}\n", a.name, a.at));
        if let Some(v) = a.vma {
            s.push_str(&format!("vma = {v:#x}\n"));
        }
        if let Some(w) = a.when {
            s.push_str(&format!("when = \"{}\"\n", w.as_str()));
        }
    }
    s
}

/// `emit_sound_blob` refuses an overlay it cannot read, and writes nothing.
#[test]
fn the_emit_binary_refuses_a_missing_overlay() {
    let Some(aeon) = reference_tree_for_profile(&native::sonic4_profile(false)) else { return };
    let scratch = Scratch::new("missing");
    let out = scratch.0.join("generated");
    let missing = scratch.0.join("no-such-anchors.toml");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_emit_sound_blob"))
        .arg("--aeon")
        .arg(&aeon)
        .arg("--out-dir")
        .arg(&out)
        .arg("--anchor-overlay")
        .arg(&missing)
        .output()
        .expect("run emit_sound_blob");
    let err = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(1), "{err}");
    assert!(err.contains("[map.overlay-read]") && err.contains("no-such-anchors.toml"), "{err}");
    assert!(!out.exists(), "a refused emit wrote {}", out.display());
}

/// `emit_sound_blob` refuses the switch given twice (a usage error, exit 2), and writes
/// nothing.
#[test]
fn the_emit_binary_refuses_two_overlays() {
    let Some(aeon) = reference_tree_for_profile(&native::sonic4_profile(false)) else { return };
    let scratch = Scratch::new("twice");
    let out = scratch.0.join("generated");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_emit_sound_blob"))
        .arg("--aeon")
        .arg(&aeon)
        .arg("--out-dir")
        .arg(&out)
        .args(["--anchor-overlay", "a.toml", "--anchor-overlay", "b.toml"])
        .output()
        .expect("run emit_sound_blob");
    let err = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(2), "{err}");
    assert!(err.contains("--anchor-overlay takes one file"), "{err}");
    assert!(!out.exists(), "a refused emit wrote {}", out.display());
}
