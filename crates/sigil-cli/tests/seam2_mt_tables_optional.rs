//! seam-2 accepts an `mt_bank.emp` with or without the song pointer tables.
//!
//! Since song bank 2 step 1 the reference `mt_bank.emp` carries no `SongTable` /
//! `SongPatchTable`: they are ordinary label arrays in `sfx_bank_blob.emp`, and seam-2
//! measures the whole MT bank to predict the `Sfx_33` base that follows it. seam-2 still
//! accepts the older shape, the two tables as the section's last items, and splits it
//! into a body plus the two tables; it refuses a module carrying one table without the
//! other (`[sound.mt-half-tables]`).
//!
//! THE WITH-TABLES INPUT IS DERIVED FROM THE REFERENCE, NOT TYPED. Each test copies the
//! reference tree's `engine/` and `games/sonic4/` into a scratch directory under
//! `CARGO_TARGET_TMPDIR` and appends the named table items to that copy's `mt_bank`
//! section, each `4 * SONG_COUNT` zero bytes with `SONG_COUNT` resolved per shape from
//! aeon's song-id authority. Every other byte of the module is the real one. The tables'
//! CONTENT is not what these tests measure (seam-2 only locates and splits them), so the
//! zero cells stand in for aeon's pointers. If the reference module carries a table item
//! itself, the derivation refuses to append a second one and the test fails naming that.
//!
//! The expectations are derived too: the real bank must equal the with-tables bank's
//! bytes up to where `SongTable` begins, shorter by the two tables, and each SFX base
//! must be the walk's own `packed_chained_base` over the bank length it follows.
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

/// The names of the `data` items in `mt_bank.emp`'s `mt_bank` section, and the byte
/// offset of the section's closing brace, by the parser's own spans.
fn section_items(src: &str) -> (Vec<String>, usize) {
    let (file, diags) = parse_str(src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the {MT_BANK_REL} under test does not parse: {diags:?}"
    );
    for item in &file.items {
        let Item::Section(sec) = item else { continue };
        if sec.name != "mt_bank" {
            continue;
        }
        let names = sec
            .items
            .iter()
            .filter_map(|i| if let Item::Data(d) = i { Some(d.name.clone()) } else { None })
            .collect();
        let close = sec.span.end as usize - 1;
        assert_eq!(&src[close..=close], "}", "the mt_bank section span does not end at its brace");
        return (names, close);
    }
    panic!("{MT_BANK_REL} has no `mt_bank` section");
}

/// `src` with each of `names` appended as the `mt_bank` section's last items, sized
/// `4 * SONG_COUNT` per shape. None may already be there: appending a second item of
/// the same name would test a module aeon never wrote.
fn with_items(src: &str, names: &[&str], count_plain: i64, count_debug: i64) -> String {
    let (present, close) = section_items(src);
    for name in names {
        assert!(
            !present.iter().any(|p| p == name),
            "the reference {MT_BANK_REL} already carries `data {name}` in section mt_bank; \
             this test derives its with-tables input by appending it, so it cannot build that input"
        );
    }
    let mut add = String::new();
    for name in names {
        add.push_str(&format!(
            "    data {name} = if DEBUG == 1 {{ bytes(comptime for i in 0..{} {{ 0 }}) }} else {{ bytes(comptime for i in 0..{} {{ 0 }}) }}\n",
            4 * count_debug,
            4 * count_plain
        ));
    }
    let mut out = src.to_string();
    out.insert_str(close, &add);
    let (after, _) = section_items(&out);
    for name in names {
        assert_eq!(after.iter().filter(|p| p == name).count(), 1, "the append did not land `{name}`");
    }
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
/// seam-2 emitters read) whose `mt_bank.emp` has the `names` items appended.
fn tree_with(real: &Path, names: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch dir on disk");
    let root = tmp.path().to_path_buf();
    copy_tree(&real.join("engine"), &root.join("engine"));
    copy_tree(&real.join("games/sonic4"), &root.join("games/sonic4"));
    let count = |debug| song_id_carrier(real, debug).expect("SONG_COUNT").song_count;
    let src = std::fs::read_to_string(real.join(MT_BANK_REL)).unwrap();
    std::fs::write(root.join(MT_BANK_REL), with_items(&src, names, count(false), count(true))).unwrap();
    (tmp, root)
}

fn reference() -> Option<PathBuf> {
    reference_tree(&[MT_BANK_REL, "games/sonic4/map.toml", "engine/sound/z80_sound_driver.emp"])
}

/// The reference bank reports no tables and is exactly the with-tables bank's body: its
/// bytes up to where `SongTable` begins, shorter by the two tables.
#[test]
fn an_mt_bank_without_song_tables_is_the_real_body() {
    let Some(real) = reference() else { return };
    let (_tmp, doctored) = tree_with(&real, &TABLES);
    for debug in [false, true] {
        let with =
            emit_mt_bank(&doctored, debug).unwrap_or_else(|e| panic!("with-tables emit_mt_bank({debug}): {e}"));
        let t = with.tables.as_ref().unwrap_or_else(|| {
            panic!("the with-tables bank reports no song tables, so it is not the with-tables control")
        });
        let without = emit_mt_bank(&real, debug).unwrap_or_else(|e| panic!("real emit_mt_bank({debug}): {e}"));
        assert!(without.tables.is_none(), "the reference bank carries no table items and reports none");

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
            "debug={debug}: the no-tables bank must be the with-tables body byte for byte"
        );
    }
}

/// seam-2 predicts the SFX base from the MT length in both shapes: every base upstream
/// of the MT bank is unchanged by the tables, and each SFX base is the walk's own
/// rounding over the bank it follows.
#[test]
fn sound_layout_without_song_tables_predicts_sfx_from_the_shorter_bank() {
    let Some(real) = reference() else { return };
    let (_tmp, doctored) = tree_with(&real, &TABLES);
    let with = sound_layout(&doctored).expect("with-tables sound_layout");
    let without = sound_layout(&real).expect("real sound_layout");

    assert_eq!(without.dac_blip_lma, with.dac_blip_lma);
    assert_eq!(without.dac_shared_lma, with.dac_shared_lma);
    assert_eq!(without.sound_tables_z80_lma, with.sound_tables_z80_lma);
    assert_eq!(without.pitchtable_lma, with.pitchtable_lma);
    assert_eq!(without.sfx_win_tab_lma, with.sfx_win_tab_lma);
    assert_eq!(without.seq_opcode_tab_lma, with.seq_opcode_tab_lma);
    assert_eq!(without.dac_sample_tab_lma, with.dac_sample_tab_lma);
    assert_eq!(without.mt_bank_lma, with.mt_bank_lma);

    for (debug, got, with_sfx) in [
        (false, without.sfx_bank_lma_plain, with.sfx_bank_lma_plain),
        (true, without.sfx_bank_lma_debug, with.sfx_bank_lma_debug),
    ] {
        let len = emit_mt_bank(&real, debug).expect("real emit_mt_bank").bytes.len() as u32;
        let want = packed_chained_base(without.mt_bank_lma + len, "Sfx_33").expect("Sfx_33 is declared");
        assert_eq!(got, want, "debug={debug}: the SFX base must follow the no-tables MT length");
        assert_ne!(got, with_sfx, "debug={debug}: the tables must move the SFX base (non-vacuity)");
    }
}

/// With no tables, the emitted body is the whole bank and no table artifact is left
/// behind: a stale `mt_songtable*.bin` from an earlier run is removed, so a consumer
/// that still embeds one fails on a missing file rather than linking old pointers.
#[test]
fn mt_artifacts_without_song_tables_are_the_whole_bank_and_no_tables() {
    let Some(real) = reference() else { return };
    let (_tmp, copy) = tree_with(&real, &[]);
    let gen = copy.join("engine/sound/generated");
    std::fs::create_dir_all(&gen).unwrap();
    let stale =
        ["mt_songtable.bin", "mt_songpatchtable.bin", "mt_songtable_debug.bin", "mt_songpatchtable_debug.bin"];
    for name in stale {
        std::fs::write(gen.join(name), b"stale").unwrap();
    }

    emit_mt_artifacts(&copy, &gen).unwrap_or_else(|e| panic!("no-tables emit_mt_artifacts: {e}"));

    for (debug, body) in [(false, "mt_bank_body.bin"), (true, "mt_bank_body_debug.bin")] {
        let bank = emit_mt_bank(&copy, debug).expect("no-tables emit_mt_bank");
        assert!(bank.tables.is_none(), "the reference bank carries no table items");
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
    for (only, missing) in [("SongTable", "SongPatchTable"), ("SongPatchTable", "SongTable")] {
        let (_tmp, doctored) = tree_with(&real, &[only]);
        let err = match emit_mt_bank(&doctored, false) {
            Ok(_) => panic!("a bank with `{only}` but without its partner must be refused"),
            Err(e) => e,
        };
        assert!(err.contains(&format!("`{missing}`")), "the refusal must name `{missing}`: {err}");
        assert!(err.contains("[sound.mt-half-tables]"), "the refusal must carry its code: {err}");
    }
}
