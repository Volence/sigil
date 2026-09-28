//! Parcel K4 inc-5 Stage 3 (P2 MT probe): the Moving-Trucks streaming bank,
//! region-level byte gate against the reference ROM.
//!
//! `games/sonic4/data/sound/mt_bank.emp` (module `games.sonic4.mt_bank_blob`) is the
//! ROM-placed module: its `mt_bank` section sits at the `Song_MovingTrucks` map row,
//! non-phased, directly above the engine-table head, and embeds the song blobs by
//! aeon-root-relative paths, as the ROM build lowers it. SHAPE-DEPENDENT size (debug
//! adds DrumTest + HCZ2).
//!
//! Every address and length above lives in `pins::MT_BANK_BLOB` (and
//! `pins::SOUNDBANKHEAD` for the head) — regenerate via `repin`. They are
//! deliberately not restated here: a bound copied into prose is executed by
//! nothing, so nothing can go red when it rots.
//!
//! The split tests are EMIT-FIRST: the seam-2 artifacts they read are gitignored
//! build output, so they run `ensure_generated` first, then compare.
//!
//! REFERENCE-DEPENDENT: the sources and the reference ROMs live in the sibling
//! `aeon` tree (`AEON_DIR`, or `EMPYREAN_SUITE_ROOT`). Absent,
//! every test here SKIPS green — unless `SIGIL_STRICT_GATE=1` makes a missing
//! reference a hard failure, so the pre-merge run cannot skip a gate. The split
//! tests guard BEFORE `ensure_generated`, which WRITES its artifacts into the
//! tree.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test mt_bank_port
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::test_support::{aeon_dir as aeon_root, reference_tree, strict_gate};
use sigil_harness::{native, pins};
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use std::path::{Path, PathBuf};

/// The reference tree the gates read: `mt_bank.emp` (the ROM-placed module and the
/// seam-2 lowering) and the resident blob head `ensure_generated` re-emits alongside it. `None`
/// skips — checked BEFORE `ensure_generated`, which writes into the tree.
fn mt_tree() -> Option<PathBuf> {
    reference_tree(&[
        "games/sonic4/data/sound/mt_bank.emp",
        "engine/sound/z80_sound_driver.emp",
    ])
}

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn compile(base: u32, len: usize, debug: bool) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    let dir = aeon.join("games/sonic4/data/sound");
    let path = dir.join("mt_bank.emp");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read: {e}"));
    let (file, pd) = parse_str(&src);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {pd:?}");
    // The base the module's own embed literals name (the aeon root, as the ROM build
    // resolves them, or the module's directory), as seam-2 picks it.
    let root = sigil_harness::seam2::mt_bank_embed_root(&aeon, &dir, &src)
        .unwrap_or_else(|e| panic!("mt_bank.emp embed root: {e}"));
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(root.clone()),
        embed_base: Some(root),
        // The bank selects on DEBUG (debug adds DrumTest + HCZ2).
        defines: vec![("DEBUG".to_string(), if debug { 1 } else { 0 })],
    };
    let (module, ld) = lower_module(&file, &opts);
    assert!(ld.iter().all(|d| d.level != sigil_span::Level::Error), "lower: {ld:?}");
    // The module's top-level ensures open a zero-byte default-section carrier, which
    // needs a home as well as `mt_bank` itself.
    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n\
         [[region]]\nname = \"mt_bank\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map).expect("map");
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "place: {pd:?}");
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve: {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new()).unwrap_or_else(|d| panic!("link: {d:?}"))
}

fn gate(debug: bool, rom_name: &str) {
    let _guard = LOCK.lock().unwrap();
    let rom_path = aeon_root().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let base = if debug { pins::MT_BANK_BLOB.debug_base } else { pins::MT_BANK_BLOB.plain_base };
    let len = if debug { pins::MT_BANK_BLOB.debug_len } else { pins::MT_BANK_BLOB.plain_len };

    let linked = compile(base, len, debug);
    let sec = linked.section("mt_bank").expect("linked image must carry mt_bank");
    assert_eq!(sec.bytes.len(), len, "mt_bank must emit {len:#x} bytes");
    let expected = &refrom[base as usize..base as usize + len];
    if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
        panic!(
            "mt_bank ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
            if debug { "debug" } else { "plain" },
            &sec.bytes[i.saturating_sub(4)..(i + 8).min(len)],
            &expected[i.saturating_sub(4)..(i + 8).min(len)]
        );
    }
}

#[test]
fn mt_bank_matches_reference() {
    gate(false, "s4.bin");
}

#[test]
fn mt_bank_debug_matches_reference() {
    gate(true, "s4.debug.bin");
}

/// The split IDENTITY bar: the emitted artifacts reassemble to the un-split lowering
/// of `mt_bank.emp`, both shapes. With the song tables in the module that is body +
/// SongTable + SongPatchTable; without them the body is the whole bank and no table
/// artifact exists. Ties the on-disk split back to its source of truth
/// (`emit_mt_bank`), so a split-boundary bug (wrong offset) fails here loudly.
fn split_reassembles(aeon: &Path, debug: bool) {
    let _guard = LOCK.lock().unwrap();
    native::ensure_generated(aeon);
    let gen = aeon.join("engine/sound/generated");
    let mt = sigil_harness::seam2::emit_mt_bank(aeon, debug)
        .unwrap_or_else(|e| panic!("emit_mt_bank({debug}): {e}"));
    let (body, st, spt) = if debug {
        ("mt_bank_body_debug.bin", "mt_songtable_debug.bin", "mt_songpatchtable_debug.bin")
    } else {
        ("mt_bank_body.bin", "mt_songtable.bin", "mt_songpatchtable.bin")
    };
    let mut reassembled = std::fs::read(gen.join(body)).expect("read body");
    if mt.tables.is_some() {
        reassembled.extend(std::fs::read(gen.join(st)).expect("read songtable"));
        reassembled.extend(std::fs::read(gen.join(spt)).expect("read songpatchtable"));
    } else {
        for name in [st, spt] {
            assert!(!gen.join(name).exists(), "{name} exists although mt_bank carries no song tables");
        }
    }
    assert_eq!(
        reassembled, mt.bytes,
        "the emitted artifacts ({}) must reassemble to the un-split lowering",
        if debug { "debug" } else { "plain" }
    );
}

#[test]
fn split_artifacts_reassemble_to_unsplit_blob_plain() {
    let Some(aeon) = mt_tree() else { return };
    split_reassembles(&aeon, false);
}

#[test]
fn split_artifacts_reassemble_to_unsplit_blob_debug() {
    let Some(aeon) = mt_tree() else { return };
    split_reassembles(&aeon, true);
}
