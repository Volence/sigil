//! Parcel K4 inc-5 Stage 2 (P2 DAC probe) — the DAC sample banks, region-level byte
//! gate, EMIT-FIRST.
//!
//! The flat BINCLUDE island in `games/sonic4/main.asm`'s `gameSoundDataIncludes`
//! (dac_blip_bank.bin @ $48000 + dac_shared_bank.bin @ $50000) is now a native
//! `.emp` `embed()` section (`games.sonic4.dac_banks`) — the P2 path: it embeds the
//! seam-2-emitted artifacts (untouched emit-tool architecture) at their fixed bank
//! LMAs, one $8000 window apart (the intra-section `align $8000` is the inter-bank
//! pad). Head label Dac_Temp_Blip; shape-invariant.
//!
//! EMIT-FIRST (the boot_data pattern): the embedded `.bin` are gitignored BUILD
//! ARTIFACTS, so the gate runs `ensure_generated` FIRST (the build's own contract),
//! then compares. This is the emit-first golden gate spec §6 requires.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test dac_bank_port
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

// The emit step touches the shared engine/sound/generated dir.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn compile(base: u32, len: usize) -> sigil_link::LinkedImage {
    let aeon = aeon_root();
    // EMIT-FIRST: the embedded dac_blip_bank.bin / dac_shared_bank.bin are gitignored
    // build artifacts — regenerate them (the build's contract) before the embed lowers.
    native::ensure_generated(&aeon);
    let path = aeon.join("games/sonic4/data/sound/dac_banks.emp");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read: {e}"));
    let (file, pd) = parse_str(&src);
    assert!(pd.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {pd:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: Some(aeon.clone()),
        defines: vec![],
    };
    let (module, ld) = lower_module(&file, &opts);
    assert!(ld.iter().all(|d| d.level != sigil_span::Level::Error), "lower: {ld:?}");
    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"dac_banks\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
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
    let base = if debug { pins::DAC_BANKS.debug_base } else { pins::DAC_BANKS.plain_base };
    let len = pins::DAC_BANKS.plain_len;
    assert_eq!(pins::DAC_BANKS.debug_len, len, "DAC banks len must be shape-invariant");

    let linked = compile(base, len);
    let sec = linked.section("dac_banks").expect("linked image must carry dac_banks");
    assert_eq!(sec.bytes.len(), len, "dac_banks must emit {len:#x} bytes (blip + $8000 pad + shared)");
    let expected = &refrom[base as usize..base as usize + len];
    if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
        panic!(
            "dac_banks ({}) first diff at region offset {i:#x}: got {:02x?}, expected {:02x?}",
            if debug { "debug" } else { "plain" },
            &sec.bytes[i.saturating_sub(4)..(i + 8).min(len)],
            &expected[i.saturating_sub(4)..(i + 8).min(len)]
        );
    }
}

#[test]
fn dac_banks_matches_reference() {
    gate(false, "s4.bin");
}

#[test]
fn dac_banks_debug_matches_reference() {
    gate(true, "s4.debug.bin");
}
