//! seam-2 does not require `mt_bank.emp` to carry the song pointer tables.
//!
//! Today `mt_bank.emp` ends in `SongTable` / `SongPatchTable`, and seam-2 splits its
//! lowering into a body plus the two tables. Aeon's next step moves those tables out
//! into ordinary linker-resolved label arrays in another module, so a second Z80 song
//! bank can be named from them. seam-2 must accept both shapes of `mt_bank.emp`, and in
//! the one without tables it must still measure the MT bank's length, because the
//! `Sfx_33` base it predicts follows that length.
//!
//! THE NO-TABLES INPUT IS DERIVED, NOT TYPED. Each test copies the reference tree's
//! `engine/` and `games/sonic4/` into a scratch directory under `CARGO_TARGET_TMPDIR`
//! and rewrites that copy's `mt_bank.emp` with the table items cut out by their parsed
//! spans. Every other byte of the module is the real one, so the tests follow aeon's
//! module as it changes. If the reference module no longer carries the tables, the
//! derivation has nothing to cut and the tests fail naming that, never pass on an
//! input they did not build.
//!
//! The expectations are derived too: the no-tables bank must equal the real bank's
//! bytes up to where `SongTable` began, it must be shorter by the two tables
//! (`2 * 4 * SONG_COUNT`, with `SONG_COUNT` resolved from aeon's song-id authority),
//! and the SFX base must be the walk's own `packed_chained_base` over that length.
//!
//! Reference tree: `AEON_DIR`. Absent, each test skips green outside strict mode and
//! hard-fails naming the path under `SIGIL_STRICT_GATE=1`.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::parse_str;
use sigil_harness::native::packed_chained_base;
use sigil_harness::seam2::{emit_mt_artifacts, emit_mt_bank, song_id_carrier, sound_layout};
use sigil_harness::test_support::reference_tree;
use std::path::{Path, PathBuf};

const MT_BANK_REL: &str = "games/sonic4/data/sound/mt_bank.emp";
const TABLES: [&str; 2] = ["SongTable", "SongPatchTable"];

/// The byte range of each `data <name>` item in `mt_bank.emp`'s `mt_bank` section, by
/// the parser's own spans.
fn item_spans(src: &str, names: &[&str]) -> Vec<(String, usize, usize)> {
    let (file, diags) = parse_str(src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the reference mt_bank.emp does not parse: {diags:?}"
    );
    let mut out = Vec::new();
    for item in &file.items {
        let Item::Section(sec) = item else { continue };
        if sec.name != "mt_bank" {
            continue;
        }
        for inner in &sec.items {
            if let Item::Data(d) = inner {
                if names.contains(&d.name.as_str()) {
                    out.push((d.name.clone(), d.span.start as usize, d.span.end as usize));
                }
            }
        }
    }
    out
}

/// `src` with the `data` items named `names` cut out of the `mt_bank` section. Every
/// named item must be present exactly once: a derivation that found nothing to cut
/// would hand the test the unchanged module and let it pass on the wrong input.
fn without_items(src: &str, names: &[&str]) -> String {
    let mut spans = item_spans(src, names);
    for name in names {
        let n = spans.iter().filter(|(s, _, _)| s == name).count();
        assert_eq!(
            n, 1,
            "the reference {MT_BANK_REL} carries `data {name}` {n} times in section mt_bank; \
             this test derives its input by cutting it out, so it cannot build that input"
        );
    }
    spans.sort_by_key(|(_, start, _)| std::cmp::Reverse(*start));
    let mut out = src.to_string();
    for (name, start, end) in spans {
        let cut = &out[start..end];
        assert!(
            cut.starts_with(&format!("data {name}")),
            "the span of `{name}` does not start at its declaration: {cut:?}"
        );
        out.replace_range(start..end, "");
    }
    let left = item_spans(&out, names);
    assert!(left.is_empty(), "the cut left {left:?} in the module");
    out
}

/// Copy the directory tree at `from` to `to`, real files and directories only.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
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

/// A scratch copy of the reference tree's `engine/` and `games/sonic4/` (everything the
/// seam-2 emitters read) whose `mt_bank.emp` has the `names` items cut out.
fn tree_without(real: &Path, names: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch dir on disk");
    let root = tmp.path().to_path_buf();
    copy_tree(&real.join("engine"), &root.join("engine"));
    copy_tree(&real.join("games/sonic4"), &root.join("games/sonic4"));
    let src = std::fs::read_to_string(real.join(MT_BANK_REL)).unwrap();
    std::fs::write(root.join(MT_BANK_REL), without_items(&src, names)).unwrap();
    (tmp, root)
}

fn reference() -> Option<PathBuf> {
    reference_tree(&[MT_BANK_REL, "games/sonic4/map.toml", "engine/sound/z80_sound_driver.emp"])
}

/// The no-tables bank lowers, reports no tables, and is exactly the real bank's body:
/// the real bytes up to where `SongTable` began, shorter by the two tables.
#[test]
fn an_mt_bank_without_song_tables_is_the_real_body() {
    let Some(real) = reference() else { return };
    let (_tmp, doctored) = tree_without(&real, &TABLES);
    for debug in [false, true] {
        let with = emit_mt_bank(&real, debug).unwrap_or_else(|e| panic!("real emit_mt_bank({debug}): {e}"));
        let t = with.tables.as_ref().unwrap_or_else(|| {
            panic!("the reference mt_bank carries no song tables, so it is not the with-tables control")
        });
        let without =
            emit_mt_bank(&doctored, debug).unwrap_or_else(|e| panic!("no-tables emit_mt_bank({debug}): {e}"));
        assert!(without.tables.is_none(), "a bank without the table items reports none");

        let song_count = song_id_carrier(&real, debug).expect("SONG_COUNT").song_count as usize;
        assert!(song_count > 0, "SONG_COUNT resolved to {song_count}, the length check would be empty");
        assert_eq!(
            without.bytes.len(),
            with.bytes.len() - 2 * 4 * song_count,
            "debug={debug}: the no-tables bank must be shorter by the two SONG_COUNT*4 tables"
        );
        assert_eq!(
            without.bytes,
            with.bytes[..t.song_table_off],
            "debug={debug}: the no-tables bank must be the real body byte for byte"
        );
    }
}

/// seam-2 still predicts the SFX base from the MT length when the tables are gone: every
/// base upstream of the MT bank is unchanged, and each SFX base is the walk's own
/// rounding over the shorter bank.
#[test]
fn sound_layout_without_song_tables_predicts_sfx_from_the_shorter_bank() {
    let Some(real) = reference() else { return };
    let (_tmp, doctored) = tree_without(&real, &TABLES);
    let with = sound_layout(&real).expect("real sound_layout");
    let without = sound_layout(&doctored).expect("no-tables sound_layout");

    assert_eq!(without.dac_blip_lma, with.dac_blip_lma);
    assert_eq!(without.dac_shared_lma, with.dac_shared_lma);
    assert_eq!(without.sound_tables_z80_lma, with.sound_tables_z80_lma);
    assert_eq!(without.pitchtable_lma, with.pitchtable_lma);
    assert_eq!(without.sfx_win_tab_lma, with.sfx_win_tab_lma);
    assert_eq!(without.seq_opcode_tab_lma, with.seq_opcode_tab_lma);
    assert_eq!(without.dac_sample_tab_lma, with.dac_sample_tab_lma);
    assert_eq!(without.mt_bank_lma, with.mt_bank_lma);

    for (debug, got, real_sfx) in [
        (false, without.sfx_bank_lma_plain, with.sfx_bank_lma_plain),
        (true, without.sfx_bank_lma_debug, with.sfx_bank_lma_debug),
    ] {
        let len = emit_mt_bank(&doctored, debug).expect("no-tables emit_mt_bank").bytes.len() as u32;
        let want = packed_chained_base(without.mt_bank_lma + len, "Sfx_33").expect("Sfx_33 is declared");
        assert_eq!(got, want, "debug={debug}: the SFX base must follow the no-tables MT length");
        assert_ne!(got, real_sfx, "debug={debug}: dropping the tables must move the SFX base (non-vacuity)");
    }
}

/// With no tables, the emitted body is the whole bank and no table artifact is left
/// behind: a stale `mt_songtable*.bin` from an earlier run is removed, so a consumer
/// that still embeds one fails on a missing file rather than linking old pointers.
#[test]
fn mt_artifacts_without_song_tables_are_the_whole_bank_and_no_tables() {
    let Some(real) = reference() else { return };
    let (_tmp, doctored) = tree_without(&real, &TABLES);
    let gen = doctored.join("engine/sound/generated");
    std::fs::create_dir_all(&gen).unwrap();
    let stale =
        ["mt_songtable.bin", "mt_songpatchtable.bin", "mt_songtable_debug.bin", "mt_songpatchtable_debug.bin"];
    for name in stale {
        std::fs::write(gen.join(name), b"stale").unwrap();
    }

    emit_mt_artifacts(&doctored, &gen).unwrap_or_else(|e| panic!("no-tables emit_mt_artifacts: {e}"));

    for (debug, body) in [(false, "mt_bank_body.bin"), (true, "mt_bank_body_debug.bin")] {
        let bank = emit_mt_bank(&doctored, debug).expect("no-tables emit_mt_bank");
        let written = std::fs::read(gen.join(body)).unwrap_or_else(|e| panic!("read {body}: {e}"));
        assert_eq!(written, bank.bytes, "{body} must be the whole no-tables bank");
    }
    for name in stale {
        assert!(!gen.join(name).exists(), "stale table artifact {name} survived a no-tables emit");
    }
}

/// The two tables are parallel: a bank carrying one without the other is refused, and
/// the refusal names the one that is missing.
#[test]
fn a_bank_with_one_song_table_is_refused_naming_the_other() {
    let Some(real) = reference() else { return };
    for (cut, missing) in [("SongPatchTable", "SongPatchTable"), ("SongTable", "SongTable")] {
        let (_tmp, doctored) = tree_without(&real, &[cut]);
        let err = match emit_mt_bank(&doctored, false) {
            Ok(_) => panic!("a bank without `{cut}` but with its partner must be refused"),
            Err(e) => e,
        };
        assert!(err.contains(&format!("`{missing}`")), "the refusal must name `{missing}`: {err}");
        assert!(err.contains("[sound.mt-half-tables]"), "the refusal must carry its code: {err}");
    }
}
