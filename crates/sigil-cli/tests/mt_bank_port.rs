//! Parcel K4 inc-5 Stage 3 (P2 MT probe) — the Moving-Trucks streaming bank body,
//! region-level byte gate, EMIT-FIRST.
//!
//! The BINCLUDE at $58607 in `games/sonic4/main.asm` (the MT body, after the phased
//! soundBankHead) is now a native `.emp` `embed()` section (`games.sonic4.mt_bank_blob`)
//! — the P2 path: it embeds the seam-2-emitted three-way split (mt_bank_body +
//! mt_songtable + mt_songpatchtable, per shape; untouched emit-tool architecture) at
//! its fixed LMA $58607 (non-phased; the soundBankHead above it is $607 both shapes).
//! Head label Song_MovingTrucks; SHAPE-DEPENDENT size (debug adds DrumTest + HCZ2):
//! 0x34E1 plain / 0x4F33 debug. SongTable/SongPatchTable are the last two contiguous
//! members — native section labels the whole-ROM link resolves for sound_api.emp.
//!
//! EMIT-FIRST: the embedded `.bin` are gitignored build artifacts, so the gate runs
//! `ensure_generated` FIRST, then compares.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test mt_bank_port
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::{native, pins};
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use std::path::PathBuf;

fn aeon_root() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn compile(base: u32, len: usize, debug: bool) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    // EMIT-FIRST: the embedded mt_bank{,_debug}.bin are gitignored build artifacts.
    native::ensure_generated(&aeon);
    let path = aeon.join("games/sonic4/data/sound/mt_bank_blob.emp");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read: {e}"));
    let (file, pd) = parse_str(&src);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {pd:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: Some(aeon.clone()),
        // The blob selects on DEBUG (debug adds DrumTest + HCZ2).
        defines: vec![("DEBUG".to_string(), if debug { 1 } else { 0 })],
    };
    let (module, ld) = lower_module(&file, &opts);
    assert!(ld.iter().all(|d| d.level != sigil_span::Level::Error), "lower: {ld:?}");
    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"mt_bank_blob\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
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
    let sec = linked.section("mt_bank_blob").expect("linked image must carry mt_bank_blob");
    assert_eq!(sec.bytes.len(), len, "mt_bank_blob must emit {len:#x} bytes");
    let expected = &refrom[base as usize..base as usize + len];
    if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
        panic!(
            "mt_bank_blob ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
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

/// The split IDENTITY bar: the three emitted artifacts (body + SongTable +
/// SongPatchTable) concatenated are byte-identical to the un-split lowering of
/// `mt_bank.emp`, both shapes. Ties the on-disk split back to its source of truth
/// (`emit_mt_bank`), so a split-boundary bug (wrong offset) fails here loudly.
fn split_reassembles(debug: bool) {
    let _guard = LOCK.lock().unwrap();
    let aeon = aeon_root();
    native::ensure_generated(&aeon);
    let gen = aeon.join("engine/sound/generated");
    let mt = sigil_harness::seam2::emit_mt_bank(&aeon, debug)
        .unwrap_or_else(|e| panic!("emit_mt_bank({debug}): {e}"));
    let (body, st, spt) = if debug {
        ("mt_bank_body_debug.bin", "mt_songtable_debug.bin", "mt_songpatchtable_debug.bin")
    } else {
        ("mt_bank_body.bin", "mt_songtable.bin", "mt_songpatchtable.bin")
    };
    let mut reassembled = std::fs::read(gen.join(body)).expect("read body");
    reassembled.extend(std::fs::read(gen.join(st)).expect("read songtable"));
    reassembled.extend(std::fs::read(gen.join(spt)).expect("read songpatchtable"));
    assert_eq!(
        reassembled, mt.bytes,
        "body+SongTable+SongPatchTable ({}) must reassemble to the un-split lowering",
        if debug { "debug" } else { "plain" }
    );
}

#[test]
fn split_artifacts_reassemble_to_unsplit_blob_plain() {
    split_reassembles(false);
}

#[test]
fn split_artifacts_reassemble_to_unsplit_blob_debug() {
    split_reassembles(true);
}
