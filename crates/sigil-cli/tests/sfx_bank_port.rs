//! Parcel K4 inc-5 Stage 4 (P2 SFX probe) — the SFX block, region-level byte gate,
//! EMIT-FIRST.
//!
//! The BINCLUDE at $5BAE8/$5D53A in `games/sonic4/main.asm` (the SFX block, after the
//! native MT body) is now a native `.emp` `embed()` section (`games.sonic4.sfx_bank_blob`)
//! — the P2 path: it embeds the seam-2-emitted sfx_bank{,_debug}.bin at its per-shape
//! LMA. Head label Sfx_33; shape-INVARIANT size (0x748), shape-DEPENDENT start (the MT
//! body before it differs) and content (the SfxTable pointer cells hold the per-shape
//! Sfx_NN addresses). NO cross-seam labels (no surviving code reads SfxTable).
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
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
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
    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"sfx_bank_blob\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
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
    let base = if debug { pins::SFX_BANK_BLOB.debug_base } else { pins::SFX_BANK_BLOB.plain_base };
    let len = pins::SFX_BANK_BLOB.plain_len;
    assert_eq!(pins::SFX_BANK_BLOB.debug_len, len, "SFX block len must be shape-invariant");

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
