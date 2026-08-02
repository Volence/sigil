//! Parcel K4 inc-5 Stage 4b (P2 soundBankHead probe) — the engine-table bank HEAD,
//! region-level byte gate, EMIT-FIRST.
//!
//! The `soundBankHead` macro (engine/sound/sound_bank.inc — DELETED) emitted the 5
//! engine-table heads inside the `phase 08000h` bracket. This is now a native `.emp`
//! PHASE-BANK section (`games.sonic4.soundbankhead`, vma $8000 / lma $58000) embedding
//! the seam-2-emitted head artifacts (sound_tables_z80 / movingtrucks_pitchtable /
//! sfx_blob_win_tab / seq_opcode_tab / dac_sample_tab). Head label SoundTablesZ80_Head;
//! shape-INVARIANT size (0x607), shape-DEPENDENT content (SfxBlobWinTab / SeqOpcodeTable
//! differ per shape). The AS twin's `fatal` span walls are the module's comptime ensures.
//!
//! EMIT-FIRST: the embedded head `.bin` are gitignored build artifacts, so the gate
//! runs `ensure_generated` FIRST, then compares against the reference ROM @ $58000.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test soundbankhead_port
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
    // EMIT-FIRST: the embedded head .bin are gitignored build artifacts.
    native::ensure_generated(&aeon);
    let path = aeon.join("games/sonic4/data/sound/soundbankhead.emp");
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
    // The section carries its own `vma: $8000`; the map gives the LMA ($58000). The
    // top-level `const`/`ensure` items land in the default `text` section (zero bytes
    // here), which still needs a region home (the dac_port precedent).
    let map = format!(
        "fill = 0x00\n\n\
         [[region]]\nname = \"text\"\nlma_base = 0x0\nsize = 0x10\nkind = \"rom\"\n\n\
         [[region]]\nname = \"soundbankhead\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
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
    // pins::SOUNDBANKHEAD.{plain,debug}_base IS the head's LMA (T4: repin auto-detects
    // the phase-bank section and pins the LMA — the placement address — not the phase
    // VMA). No manual fixup: the pin is where the bytes physically sit + the reference
    // ROM window. The phase VMA ($8000) is the section's own `vma:` declaration in the .emp.
    let lma = if debug { pins::SOUNDBANKHEAD.debug_base } else { pins::SOUNDBANKHEAD.plain_base };
    let len = pins::SOUNDBANKHEAD.plain_len;
    assert_eq!(pins::SOUNDBANKHEAD.debug_len, len, "soundBankHead len must be shape-invariant");

    let linked = compile(lma, len, debug);
    let base = lma;
    let sec = linked.section("soundbankhead").expect("linked image must carry soundbankhead");
    assert_eq!(sec.bytes.len(), len, "soundbankhead must emit {len:#x} bytes ($8000..$8607)");
    let expected = &refrom[base as usize..base as usize + len];
    if let Some(i) = (0..len).find(|&i| sec.bytes[i] != expected[i]) {
        panic!(
            "soundbankhead ({}) first diff at region offset {i:#x} (vma ${:04x}): got {:02x?}, expected {:02x?}",
            if debug { "debug" } else { "plain" },
            0x8000 + i,
            &sec.bytes[i.saturating_sub(4)..(i + 8).min(len)],
            &expected[i.saturating_sub(4)..(i + 8).min(len)]
        );
    }
}

#[test]
fn soundbankhead_matches_reference() {
    gate(false, "s4.bin");
}

#[test]
fn soundbankhead_debug_matches_reference() {
    gate(true, "s4.debug.bin");
}

/// T4 soundness catch — the previously-latent PinnedBaked misplacement (ledger 1966).
/// A PinnedBaked re-bootstrap places every native section via `emp_map_toml`, which
/// feeds each region's pinned base straight in as its `lma_base`. When the pin held the
/// phase VMA ($8000), the soundbankhead bank placed AT $8000 instead of its true $58000
/// LMA — cosmetic under the shipped `Frozen` key (which packs from the frozen table),
/// but a genuine misplacement on this path. With the pin now holding the LMA (repin's
/// `phase_bank` derivation), the pinned bootstrap lands the bank at its load address in
/// both shapes; the phase VMA survives only as the section's own `vma: $8000`.
#[test]
fn soundbankhead_pinned_bootstrap_lands_at_lma_not_vma() {
    let _guard = LOCK.lock().unwrap();
    let aeon = aeon_root();
    if !aeon.join("s4.bin").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon tree missing at {}", aeon.display());
        }
        eprintln!("skip: aeon tree not present (set AEON_DIR)");
        return;
    }
    for debug in [false, true] {
        let resolved = native::resolve_pinned_sections(&aeon, debug)
            .unwrap_or_else(|e| panic!("resolve_pinned_sections(debug={debug}): {e}"));
        let sec = resolved
            .iter()
            .find(|s| s.name == "soundbankhead")
            .expect("the pinned layout must carry the soundbankhead section");
        assert_eq!(
            sec.lma, 0x58000,
            "phase-bank bank must LOAD at its $58000 LMA on the PinnedBaked path (not the $8000 phase VMA)"
        );
        assert_eq!(
            sec.vma_base,
            Some(0x8000),
            "the phase VMA is the section's own `vma: $8000` (labels resolve there); it is not the load address"
        );
    }
}
