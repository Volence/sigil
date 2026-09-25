//! `sigil build --anchor-overlay` at the binary: an unreadable overlay is refused
//! before any build work, and a build under an overlay names the overlay file in its
//! listing's source digest. Overlay builds run in a shadow tree, because a build writes
//! its sound artifacts into the tree it builds.

use sigil_harness::native;
use sigil_harness::test_support::{reference_tree_for_profile, shadow_aeon_tree};
use std::process::Command;

const SIGIL: &str = env!("CARGO_BIN_EXE_sigil");

/// A scratch directory for one run, removed on drop.
struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("sigil-build-overlay-{tag}-{}", std::process::id()));
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

#[test]
fn an_unreadable_overlay_is_refused_before_building() {
    let Some(aeon) = reference_tree_for_profile(&native::sonic4_profile(false)) else { return };
    let scratch = Scratch::new("missing");
    let missing = scratch.0.join("no-such-anchors.toml");
    let rom = scratch.0.join("r.bin");
    for extra in [&["--check"][..], &["-o", rom.to_str().unwrap()]] {
        let run = Command::new(SIGIL)
            .args(["build", "--aeon"])
            .arg(&aeon)
            .args(["--game", "sonic4"])
            .args(extra)
            .arg("--anchor-overlay")
            .arg(&missing)
            .output()
            .expect("run sigil build");
        let err = String::from_utf8_lossy(&run.stderr);
        assert_eq!(run.status.code(), Some(1), "{extra:?}: {err}");
        assert!(err.contains("[map.overlay-read]") && err.contains("no-such-anchors.toml"), "{extra:?}: {err}");
        assert!(!rom.exists(), "{extra:?}: a refused build wrote {}", rom.display());
    }
}

#[test]
fn an_overlay_build_names_the_overlay_in_its_source_digest() {
    let Some(aeon) = reference_tree_for_profile(&native::sonic4_profile(false)) else { return };
    let src = std::fs::read_to_string(aeon.join("games/sonic4/map.toml")).expect("read map.toml");
    let pmap = sigil_harness::map_placement::load_placement_map(&src).expect("map");
    let at = |n: &str| pmap.anchors_for(true).find(|a| a.name == n).unwrap_or_else(|| panic!("{n}")).at;
    let (dac, snd) = (at("dac_banks"), at("sound_bank"));
    let (to_dac, to_snd) = (snd, snd + (snd - dac));
    let shadow = shadow_aeon_tree(&aeon, &[]).expect("shadow tree");
    let scratch = Scratch::new("digest");
    let overlay = scratch.0.join("anchors.toml");
    std::fs::write(
        &overlay,
        format!(
            "[[anchor]]\nname = \"dac_banks\"\nat = {to_dac:#x}\nwhen = \"sound_on\"\n\n\
             [[anchor]]\nname = \"sound_bank\"\nat = {to_snd:#x}\nvma = 0x8000\nwhen = \"sound_on\"\n"
        ),
    )
    .expect("write overlay");
    let (rom, lst) = (scratch.0.join("r.bin"), scratch.0.join("r.lst"));
    let run = Command::new(SIGIL)
        .args(["build", "--aeon"])
        .arg(shadow.root())
        .args(["--game", "sonic4", "-o"])
        .arg(&rom)
        .arg("--emit-lst")
        .arg(&lst)
        .arg("--anchor-overlay")
        .arg(&overlay)
        .output()
        .expect("run sigil build");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let listing = std::fs::read_to_string(&lst).expect("read listing");
    let canon = std::fs::canonicalize(&overlay).expect("canonical overlay path");
    let name = canon.to_str().unwrap().trim_start_matches('/');
    // A read row with the bytes the build consumed: the overlay's own CRC-32 and size.
    let bytes = std::fs::read(&overlay).expect("read overlay");
    let row = format!("DIGEST-READ crc={:08x} size={} ", native::crc32(&bytes), bytes.len());
    assert!(
        listing.lines().any(|l| l.starts_with(&row) && l.ends_with(&format!("path={name}"))),
        "the source digest has no `{row}... path={name}` row"
    );
}
