//! Parcel K4 inc-5 Stage 4b (P2 soundBankHead probe) — the engine-table bank HEAD,
//! region-level byte gate, EMIT-FIRST.
//!
//! The `soundBankHead` macro (engine/sound/sound_bank.inc — DELETED) emitted the 5
//! engine-table heads inside the `phase 08000h` bracket. This is now a native `.emp`
//! PHASE-BANK section (`games.sonic4.soundbankhead`, vma $8000 / lma $58000) embedding
//! the seam-2-emitted head artifacts (sound_tables_z80 / movingtrucks_pitchtable /
//! sfx_blob_win_tab / seq_opcode_tab / dac_sample_tab). Head label SoundTablesZ80_Head;
//! shape-INVARIANT size (0x628 — sound-pkg3 v2), shape-DEPENDENT content (SfxBlobWinTab / SeqOpcodeTable
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
use sigil_harness::test_support::reference_tree_for_profile;
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

/// Serialise the gates (they share `ensure_generated`'s output dir). Poison-tolerant:
/// a sibling that panics while holding the lock must fail alone, not take every later
/// holder down with a `PoisonError` (the `native_full_rom.rs` idiom).
fn lock() -> std::sync::MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

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
    let _guard = lock();
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
    assert_eq!(sec.bytes.len(), len, "soundbankhead must emit {len:#x} bytes ($8000..$8628)");
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

/// T4 soundness catch (ledger 1966) — `pins::SOUNDBANKHEAD`'s base is the bank's LMA
/// (the placement address), NOT the $8000 phase VMA its labels resolve at. The pin is
/// what the two byte gates above window the reference ROM by, and what a pin-driven
/// placement would feed straight in as an `lma_base` (the `emp_map_toml` misplacement
/// this catch was born from): a base holding the VMA would place the bank at $8000
/// instead of its true load address.
///
/// The expectation is DERIVED, not written: the shipped (Frozen, chained) resolve —
/// the same resolve `repin` derives the pin from — says where the bank loads and that
/// it is `vma:`-windowed. The former subject, the PinnedBaked bootstrap resolve
/// (`resolve_pinned_sections`), places every `.emp` section from the REGISTRY's pins and
/// so cannot see a byte-emitting section that has no pin by design (the content-derived
/// `ojz_effects_editor_act1`, declared to the map by its `section:` row); it has no live
/// consumer (see the FIVE-REG packet), so the probe reads the layout that ships.
#[test]
fn soundbankhead_pin_is_the_lma_not_the_vma() {
    let _guard = lock();
    // No ROM is read here: the shipped resolve assembles the sonic4 shape from source,
    // so the tree this needs is the one that profile's build reads.
    let Some(aeon) = reference_tree_for_profile(&native::sonic4_profile(false)) else {
        return;
    };
    for debug in [false, true] {
        let resolved = native::resolve_canonical_sections(&aeon, debug)
            .unwrap_or_else(|e| panic!("resolve_canonical_sections(debug={debug}): {e}"));
        let sec = resolved
            .iter()
            .find(|s| s.name == "soundbankhead")
            .expect("the shipped layout must carry the soundbankhead section");
        let vma = sec
            .vma_base
            .expect("soundbankhead is a phase bank: it declares its own `vma:` window");
        assert_ne!(
            vma, sec.lma,
            "phase bank (debug={debug}): the VMA its labels resolve at must differ from the LMA it loads at"
        );
        let pin = if debug { pins::SOUNDBANKHEAD.debug_base } else { pins::SOUNDBANKHEAD.plain_base };
        assert_eq!(
            pin, sec.lma,
            "pins::SOUNDBANKHEAD base (debug={debug}) must be the bank's LMA {:#x} — where the shipped layout loads it — not its phase VMA {vma:#x}",
            sec.lma
        );
    }
}
