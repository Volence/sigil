//! A2 (step-0 soundness): a section that overflows the map region it was placed
//! into is refused, through the real CLI (`sigil emp --root <dir> --map <map>`).
//!
//! Fixture: `tests/vectors/region_probe/`, copied from the Sigil-assembler audit's
//! `z80probe` tree (reviewer B, `audit-builds/revB-r2/z80probe`). `tf/` is a three-
//! module 68k + Z80 program; its `drv_pcm` section is 7 bytes. The maps:
//! * `map_fits.toml`   (audit z1): every region holds its section. Built today.
//! * `map_small.toml`  (audit z2): `drv_pcm` is 4 B, `text` is 0x200 B, so the
//!   first ROM region holding `0x300` IS `drv_pcm`. Refused today.
//! * `map_nested.toml` (audit z3): the same 4 B `drv_pcm`, but `text` is 0x8000 B
//!   and listed first, so it held the LMA lookup and the overflow BUILT (exit 0)
//!   before A2. Refused now: a section is held to the region it was placed into.
//! * `map_when.toml`: a `when = "sound_on"` Z80 bank window (the sonic4 shape).

use std::path::{Path, PathBuf};
use std::process::Command;

fn probe() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/region_probe")
}

/// `sigil emp tf/main.emp --root tf --map <map> -o <tmp>`: (exit code, stderr).
fn emp_with_map(map: &str) -> (i32, String) {
    let dir = probe();
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .arg("emp")
        .arg(dir.join("tf/main.emp"))
        .arg("--root")
        .arg(dir.join("tf"))
        .arg("--map")
        .arg(dir.join(map))
        .arg("-o")
        .arg(tmp.path().join("out.bin"))
        .output()
        .expect("run sigil");
    (out.status.code().expect("sigil exited by signal"), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Control: the fitting map builds, so a refusal below is about the region.
#[test]
fn the_fitting_map_builds() {
    let (code, stderr) = emp_with_map("map_fits.toml");
    assert_eq!(code, 0, "{stderr}");
}

/// The first-ROM-region-by-LMA case, refused before A2 and after.
#[test]
fn a_small_region_overflow_is_refused() {
    let (code, stderr) = emp_with_map("map_small.toml");
    assert_eq!(code, 1, "{stderr}");
    assert!(
        stderr.contains("section `drv_pcm` [0x300,0x307) overflows region `drv_pcm` (ends 0x304), over by 3 bytes"),
        "{stderr}"
    );
}

/// The audit's z3: the overflow of a region nested in an earlier, wider `rom`
/// region. Exit 0 before A2 (checked against `text`); exit 1 now.
#[test]
fn nested_region_overflow_is_refused() {
    let (code, stderr) = emp_with_map("map_nested.toml");
    assert_eq!(code, 1, "the nested overflow must not build; stderr: {stderr}");
    assert!(stderr.contains("overflows region `drv_pcm` (ends 0x304), over by 3 bytes"), "{stderr}");
}

/// Harness level: a region whose `when` the shape does not meet is not in force
/// in that shape. The sonic4 Z80 sound-bank window is `when = "sound_on"`, so a
/// sound-off shape, where ordinary code may one day flow through `$B8000-$BFFFF`,
/// is not held to the window.
#[test]
fn z80_window_region_with_unmet_when_is_not_enforced() {
    let src = std::fs::read_to_string(probe().join("map_when.toml")).unwrap();
    let map = sigil_link::load_map(&src).expect("the region loader accepts `when`");
    let pmap = sigil_harness::map_placement::load_placement_map(&src).expect("placement facts");
    assert_eq!(
        pmap.region_when.get("z80_sound_bank").copied(),
        Some(sigil_harness::map_placement::When::SoundOn)
    );
    let crossing = sigil_link::LinkedImage {
        sections: vec![sigil_link::LinkedSection {
            name: "code".into(),
            lma: 0xBF000,
            bytes: vec![0x4E; 0x2000],
            region: Some("declchain0".into()),
        }],
    };

    let off = sigil_harness::map_placement::map_for_shape(&map, &pmap, false);
    assert!(off.region_by_name("z80_sound_bank").is_none(), "sound-off drops the window");
    assert!(off.region_by_name("rom").is_some(), "a region with no `when` stays");
    sigil_link::check_regions(&crossing, &off).expect("sound-off: code may cross $C0000");

    let on = sigil_harness::map_placement::map_for_shape(&map, &pmap, true);
    assert!(on.region_by_name("z80_sound_bank").is_some(), "sound-on keeps the window");
    let err = sigil_link::check_regions(&crossing, &on).expect_err("sound-on: the window holds");
    assert_eq!(
        err,
        "section `code` [0xBF000,0xC1000) overflows region `z80_sound_bank` (ends 0xC0000), over by 4096 bytes"
    );
}

/// An unknown `when` on a region is refused at load, as it is on an anchor.
#[test]
fn an_unknown_region_when_is_refused() {
    let src = "[[region]]\nname = \"z80_sound_bank\"\nlma_base = 0xB8000\nsize = 0x8000\nkind = \"z80_bank\"\nwhen = \"sound\"\n";
    let err = sigil_harness::map_placement::load_placement_map(src).unwrap_err();
    assert!(err.starts_with("[map.when-unknown] region `z80_sound_bank`"), "{err}");
}
