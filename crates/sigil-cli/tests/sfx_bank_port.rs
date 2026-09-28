//! Parcel K4 inc-5 Stage 4 (P2 SFX probe): the SFX block, region-level byte gate,
//! EMIT-FIRST.
//!
//! `games.sonic4.sfx_bank_blob` embeds the seam-2-emitted sfx_bank{,_debug}.bin at its
//! per-shape LMA (head label Sfx_33), then carries the song pointer tables
//! `SongTable` / `SongPatchTable`, `SONG_COUNT` cells each. Start, size and content are
//! all SHAPE-DEPENDENT: the MT body before it differs, the SfxTable cells hold the
//! per-shape Sfx_NN addresses, and SONG_COUNT differs per shape. The tables name the
//! song and patch labels of the song banks: those are cross-seam labels, read from the
//! reference build's listing.
//!
//! EMIT-FIRST: the embedded `.bin` are gitignored build artifacts, so the gate runs
//! `ensure_generated` FIRST, then compares.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sfx_bank_port
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::{native, pins};
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use std::path::PathBuf;

fn aeon_root() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// The labels the module names by quoted string (`["Song_MovingTrucks", ...]`, the
/// song tables' cells) that it does not define itself, each at the address the
/// reference build's listing gives it. Which names is read from the module source, so a
/// song added to the tables arrives with no edit here; a name the listing lacks for
/// this shape is left out (a shape-gated table row names it only where it exists).
fn song_table_labels(src: &str, sections: &[sigil_ir::Section], debug: bool) -> Vec<(String, u32)> {
    let defined: std::collections::HashSet<&str> =
        sections.iter().flat_map(|s| s.labels.iter().map(|l| l.name.as_str())).collect();
    let mut names: Vec<String> = Vec::new();
    for line in src.lines().map(|l| l.split("//").next().unwrap_or("")) {
        let mut rest = line;
        while let Some(i) = rest.find('"') {
            let tail = &rest[i + 1..];
            let Some(j) = tail.find('"') else { break };
            let word = &tail[..j];
            let is_ident = !word.is_empty()
                && word.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if is_ident && !defined.contains(word) && !names.iter().any(|n| n == word) {
                names.push(word.to_string());
            }
            rest = &tail[j + 1..];
        }
    }
    assert!(!names.is_empty(), "sfx_bank_blob.emp names no label by string, the table seam would be empty");
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    sigil_harness::test_support::listing_labels_if_defined(debug, &refs)
}

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn compile(base: u32, len: usize, debug: bool) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    // EMIT-FIRST: the embedded sfx_bank{,_debug}.bin are gitignored build artifacts.
    native::ensure_generated(&aeon);
    let path = aeon.join("games/sonic4/data/sound/sfx_bank_blob.emp");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read: {e}"));
    let (file, pd) = parse_str(&src);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {pd:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: Some(aeon.clone()),
        defines: vec![("DEBUG".to_string(), if debug { 1 } else { 0 })],
    };
    let (module, ld) = lower_module(&file, &opts);
    assert!(ld.iter().all(|d| d.level != sigil_span::Level::Error), "lower: {ld:?}");
    let cross_seam = song_table_labels(&src, &module.sections, debug);
    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"sfx_bank_blob\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map).expect("map");
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "place: {pd:?}");
    let mut asm = String::from("cpu 68000\n");
    for (name, addr) in &cross_seam {
        asm.push_str(&format!("{name} = ${addr:X}\n"));
    }
    asm.push_str("Stub:\n\tdc.w 0\n");
    let opts = sigil_frontend_as::Options { initial_cpu: Some(Cpu::M68000), ..Default::default() };
    for mut sec in sigil_frontend_as::assemble(&asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (cross-seam labels): {d:?}"))
        .sections
    {
        sec.lma = 0x0100_0000;
        sec.placement = sigil_ir::SectionPlacement::Pinned;
        sec.group = None;
        sections.push(sec);
    }
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
    let base = if debug { pins::SFX_BANK_BLOB.debug_base } else { pins::SFX_BANK_BLOB.plain_base };
    let len = if debug { pins::SFX_BANK_BLOB.debug_len } else { pins::SFX_BANK_BLOB.plain_len };

    let linked = compile(base, len, debug);
    let sec = linked.section("sfx_bank_blob").expect("linked image must carry sfx_bank_blob");
    assert_eq!(sec.bytes.len(), len, "sfx_bank_blob must emit {len:#x} bytes");
    let expected = &refrom[base as usize..base as usize + len];
    if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
        panic!(
            "sfx_bank_blob ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
            if debug { "debug" } else { "plain" },
            &sec.bytes[i.saturating_sub(4)..(i + 8).min(len)],
            &expected[i.saturating_sub(4)..(i + 8).min(len)]
        );
    }
}

#[test]
fn sfx_bank_matches_reference() {
    gate(false, "s4.bin");
}

#[test]
fn sfx_bank_debug_matches_reference() {
    gate(true, "s4.debug.bin");
}
