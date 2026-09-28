//! The fold gate accounts for every head seam-2 folded against. `validate_sound_fold`
//! compares each folded bank base with the base the chainer placed it at; a head the
//! layout does not place, or places behind another label, is refused by name rather than
//! left out of the comparison, and a sound-off shape that places one is refused too.
//!
//! THE EXPECTATIONS ARE DERIVED. The heads come from `seam2::FOLDED_BANK_HEADS`, the
//! constants seam-2 predicts under, and the count each shape must compare comes from the
//! shape's own sound flag. The layouts are the reference tree's own resolves
//! (`native::resolve_frozen_layout`, the placement half of the build), planted in memory
//! for the refusals: the tree on disk is never edited.
//!
//! THE SOURCE PLANTS go through the whole build on a scratch copy of the reference tree
//! under `CARGO_TARGET_TMPDIR`: a head renamed in its module (and in the map `order`, so
//! the rename is consistent everywhere a text edit can reach) must fail the build naming
//! the head. Which check refuses first is not asserted; that the build is not green is.
//!
//! Reference tree: `AEON_DIR`. Absent, each test skips green outside strict mode and
//! hard-fails naming the path under `SIGIL_STRICT_GATE=1`.

use sigil_harness::native::{self, GameProfile};
use sigil_harness::seam2::{self, FOLDED_BANK_HEADS};
use sigil_harness::test_support::reference_tree_for_profile;
use sigil_ir::{Label, Section};
use std::path::{Path, PathBuf};

fn reference(profile: &GameProfile) -> Option<PathBuf> {
    reference_tree_for_profile(profile)
}

fn resolved(aeon: &Path, profile: &GameProfile) -> Vec<Section> {
    native::resolve_frozen_layout(aeon, profile).unwrap_or_else(|e| panic!("resolve {}: {e}", profile.name))
}

/// Index of the section carrying `label` and the label's index within it.
fn locate(layout: &[Section], label: &str) -> (usize, usize) {
    layout
        .iter()
        .enumerate()
        .find_map(|(si, s)| s.labels.iter().position(|l| l.name == label).map(|li| (si, li)))
        .unwrap_or_else(|| panic!("the reference layout places no `{label}`, the plant would plant nothing"))
}

fn expect_refusal(layout: &[Section], profile: &GameProfile, aeon: &Path, code: &str, label: &str) {
    let err = native::validate_sound_fold(aeon, layout, profile)
        .map(|n| format!("Ok({n})"))
        .expect_err(&format!("{}: a layout planted for {code} on `{label}` passed the fold gate", profile.name));
    assert!(err.contains(code), "{}: expected {code}, got: {err}", profile.name);
    assert!(err.contains(&format!("`{label}`")), "{}: the refusal must name `{label}`: {err}", profile.name);
}

fn sound_on_shapes() -> Vec<GameProfile> {
    let shapes: Vec<GameProfile> =
        native::shipped_shapes().into_iter().map(|(_, p)| p).filter(|p| p.sound_on).collect();
    assert!(!shapes.is_empty(), "no shipped shape is sound-on, the plants would test nothing");
    shapes
}

/// Every shipped shape passes, comparing every folded head when it has sound and none
/// when it has not.
#[test]
fn shipped_shapes_compare_every_folded_head() {
    let shapes = native::shipped_shapes();
    assert!(shapes.iter().any(|(_, p)| p.sound_on) && shapes.iter().any(|(_, p)| !p.sound_on));
    for (name, profile) in &shapes {
        let Some(aeon) = reference(profile) else { return };
        let layout = resolved(&aeon, profile);
        let n = native::validate_sound_fold(&aeon, &layout, profile)
            .unwrap_or_else(|e| panic!("shape `{name}`: {e}"));
        let want = if profile.sound_on { FOLDED_BANK_HEADS.len() } else { 0 };
        assert_eq!(n, want, "shape `{name}` compared {n} folded heads, expected {want}");
    }
}

/// The heads the gate compares are the ones seam-2 predicted, at the bases it predicted.
#[test]
fn folded_heads_are_seam2_predictions() {
    let profile = native::sonic4_profile(false);
    let Some(aeon) = reference(&profile) else { return };
    let layout = seam2::sound_layout(&aeon).expect("layout");
    for debug in [false, true] {
        let heads = layout.folded_heads(debug);
        assert_eq!(heads.map(|(l, _)| l), FOLDED_BANK_HEADS);
        assert_eq!(heads[0].1, layout.mt_bank_lma);
        assert_eq!(heads[1].1, if debug { layout.sfx_bank_lma_debug } else { layout.sfx_bank_lma_plain });
    }
}

/// A folded head renamed in the layout is refused by name, in every sound-on shape.
#[test]
fn renamed_head_is_refused() {
    for profile in sound_on_shapes() {
        let Some(aeon) = reference(&profile) else { return };
        let layout = resolved(&aeon, &profile);
        for head in FOLDED_BANK_HEADS {
            let mut planted = layout.clone();
            let (si, li) = locate(&planted, head);
            planted[si].labels[li].name = format!("{head}_Renamed");
            expect_refusal(&planted, &profile, &aeon, "[sound.fold-head-absent]", head);
        }
    }
}

/// A folded head that stops being the first item of its section is refused by name,
/// both when another label ties it at offset 0 and when it sits past another item.
#[test]
fn head_not_first_is_refused() {
    for profile in sound_on_shapes() {
        let Some(aeon) = reference(&profile) else { return };
        let layout = resolved(&aeon, &profile);
        for head in FOLDED_BANK_HEADS {
            let (si, li) = locate(&layout, head);
            assert_eq!(layout[si].labels[li].offset, 0, "`{head}` does not head its section in the reference");

            let mut tie = layout.clone();
            tie[si].labels.insert(0, Label { name: "Planted_Ahead".into(), offset: 0 });
            expect_refusal(&tie, &profile, &aeon, "[sound.fold-head-not-first]", head);

            let mut behind = layout.clone();
            behind[si].labels[li].offset = 2;
            behind[si].labels.push(Label { name: "Planted_Ahead".into(), offset: 0 });
            expect_refusal(&behind, &profile, &aeon, "[sound.fold-head-not-first]", head);
        }
    }
}

/// The comparison itself still runs: a head section moved off its folded base is refused.
#[test]
fn moved_head_is_refused() {
    for profile in sound_on_shapes() {
        let Some(aeon) = reference(&profile) else { return };
        let layout = resolved(&aeon, &profile);
        for head in FOLDED_BANK_HEADS {
            let mut planted = layout.clone();
            let (si, _) = locate(&planted, head);
            planted[si].lma += 8;
            expect_refusal(&planted, &profile, &aeon, "[sound.fold-vs-placement]", head);
        }
    }
}

/// A sound-off shape that places a folded head is refused by name.
#[test]
fn sound_off_shape_placing_a_head_is_refused() {
    let off: Vec<GameProfile> =
        native::shipped_shapes().into_iter().map(|(_, p)| p).filter(|p| !p.sound_on).collect();
    assert!(!off.is_empty(), "no shipped shape is sound-off");
    for profile in off {
        let Some(aeon) = reference(&profile) else { return };
        let layout = resolved(&aeon, &profile);
        for head in FOLDED_BANK_HEADS {
            let mut planted = layout.clone();
            planted[0].labels.push(Label { name: head.into(), offset: 0 });
            expect_refusal(&planted, &profile, &aeon, "[sound.fold-head-sound-off]", head);
        }
    }
}

/// Copy the directory tree at `from` to `to`, real files and directories only, leaving
/// out the checkout's `.git`.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == ".git" {
            continue;
        }
        let dest = to.join(entry.file_name());
        let ty = entry.file_type().unwrap();
        if ty.is_dir() {
            copy_tree(&entry.path(), &dest);
        } else if ty.is_file() {
            std::fs::copy(entry.path(), &dest).unwrap();
        } else {
            panic!("{} is neither a file nor a directory, the copy would not be faithful", entry.path().display());
        }
    }
}

/// Replace the one whole-token definition `pub data <head> ` in the module under `dir`
/// that defines it, and the head's row in the map `order`. Returns the module edited.
fn rename_in_source(root: &Path, head: &str, to: &str) -> PathBuf {
    let sound = root.join("games/sonic4/data/sound");
    let def = format!("pub data {head} ");
    let mut hits = Vec::new();
    for entry in std::fs::read_dir(&sound).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().is_some_and(|x| x == "emp") && std::fs::read_to_string(&p).unwrap().contains(&def) {
            hits.push(p);
        }
    }
    assert_eq!(hits.len(), 1, "`{def}` must be defined by exactly one sound module, found {hits:?}");
    let module = hits.pop().unwrap();
    let src = std::fs::read_to_string(&module).unwrap();
    assert_eq!(src.matches(&def).count(), 1);
    std::fs::write(&module, src.replace(&def, &format!("pub data {to} "))).unwrap();

    let map = root.join(seam2::SOUND_PLACEMENT_MAP_REL);
    let text = std::fs::read_to_string(&map).unwrap();
    let row = format!("\"{head}\"");
    assert_eq!(text.matches(&row).count(), 1, "the map must name `{head}` in exactly one row");
    std::fs::write(&map, text.replace(&row, &format!("\"{to}\""))).unwrap();
    module
}

/// A head renamed in source on a scratch copy of the tree fails the build naming it, in
/// both sonic4 shapes.
#[test]
fn head_renamed_in_source_fails_the_build() {
    for debug in [false, true] {
        let profile = native::sonic4_profile(debug);
        let Some(real) = reference(&profile) else { return };
        for head in FOLDED_BANK_HEADS {
            let tmp = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch dir on disk");
            copy_tree(&real, tmp.path());
            let to = format!("{head}_Renamed");
            let module = rename_in_source(tmp.path(), head, &to);
            assert!(std::fs::read_to_string(&module).unwrap().contains(&format!("pub data {to} ")));
            let err = native::build_rom_chained(tmp.path(), &profile)
                .err()
                .unwrap_or_else(|| panic!("{}: renaming `{head}` in source built green", profile.name));
            assert!(err.contains(head), "{}: the refusal must name `{head}`: {err}", profile.name);
        }
    }
}
