//! seam-1 — THE RESIDENT-BLOB NATIVE LINK gates. The five sound `.emp` files link
//! as ONE native Z80 module set at VMA `$0000` / LMA `$3DE` (plain) / `$3E2`
//! (debug), with the intra-blob cross-file `extern proc` references resolving
//! INTERNALLY against the sibling sections' `pub proc` exports. The machinery lives
//! in `sigil_harness::seam1` (shared with the `emit_sound_blob` bin); this file is
//! the byte-gate + whole-ROM dual-build proofs.
//!
//! Comparands (the reference-ROM blob slice at LMA):
//!   * plain: `s4.bin[0x3DE..0x3DE+0x181C]` (6172 B)
//!   * debug: `s4.debug.bin[0x3E2..0x3E2+0x189A]` (6300 B = 6172 + $7E) — the debug
//!     base is `$3E2` (+4 upstream of BootData), NOT `$3DE`; the design §2.1/OQ-2
//!     comparand fixed the +$7E SIZE but assumed the base held (corrected here).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam1_native_link
//! ```

use sigil_harness::seam1::{
    self, blob_lma, native_blob_doctored, native_sound_blob, BLOB_LEN_DEBUG, BLOB_LEN_PLAIN,
};
use sigil_harness::{assemble_mixed_z80sound_as_side, assert_rom_matches_convsym};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

fn read_ref(name: &str) -> Option<Vec<u8>> {
    let path = aeon_dir().join(name);
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference missing: {}", path.display());
            }
            eprintln!("skip: reference ROM not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

fn assert_blob_matches(blob: &[u8], expected: &[u8], debug: bool, what: &str) {
    assert_eq!(
        blob.len(),
        expected.len(),
        "{what}: length mismatch (blob {} vs reference {})",
        blob.len(),
        expected.len()
    );
    if let Some(i) = (0..blob.len()).find(|&i| blob[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(blob.len());
        panic!(
            "{what}: first diff at blob offset {i:#x} (LMA {:#x})\n  blob[{lo:#x}..{hi:#x}]: {:02x?}\n   ref[{lo:#x}..{hi:#x}]: {:02x?}",
            blob_lma(debug) as usize + i,
            &blob[lo..hi],
            &expected[lo..hi]
        );
    }
}

// ===========================================================================
// §2.1/2.2/2.3 — the whole-blob byte gate (both shapes) + t24 non-vacuity controls.
// ===========================================================================

/// (PLAIN) the natively-linked blob == `s4.bin[0x3DE..0x3DE+0x181C]`.
#[test]
fn native_blob_matches_reference_plain() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let blob = native_sound_blob(&aeon_dir(), false).bytes;
    let base = blob_lma(false) as usize;
    let expected = &refrom[base..base + BLOB_LEN_PLAIN];
    assert_blob_matches(&blob, expected, false, "native blob (plain) vs s4.bin[$3DE..$1BFA]");
}

/// (DEBUG) the natively-linked blob == `s4.debug.bin[0x3E2..0x3E2+0x189A]` — the
/// +$7E sequencer growth EMITTED, sfx/fm/psg re-based, the driver's 9 cross-seam
/// operand bytes derived from the real imports per shape.
#[test]
fn native_blob_matches_reference_debug() {
    let Some(refrom) = read_ref("s4.debug.bin") else { return };
    let blob = native_sound_blob(&aeon_dir(), true).bytes;
    let base = blob_lma(true) as usize;
    let expected = &refrom[base..base + BLOB_LEN_DEBUG];
    assert_blob_matches(&blob, expected, true, "native blob (debug) vs s4.debug.bin[$3E2..$1C7C]");
}

/// The debug blob is exactly $7E longer than plain; both are the canonical
/// `Z80_SOUND_SIZE`.
#[test]
fn blob_lengths_are_canonical() {
    assert_eq!(BLOB_LEN_DEBUG - BLOB_LEN_PLAIN, 0x7E, "debug grows +$7E over plain");
    assert_eq!(BLOB_LEN_PLAIN, 0x181C, "plain blob is Z80_SOUND_SIZE = $181C");
}

/// t24 positive control (BANKED-CARRIER axis): a doctored `SeqOpcodeTable` must make
/// the blob DIVERGE from the reference — the byte gate is not vacuous.
#[test]
fn blob_diverges_when_banked_carrier_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let base = blob_lma(false) as usize;
    let expected = &refrom[base..base + BLOB_LEN_PLAIN];
    let doctored = native_blob_doctored(&aeon_dir(), false, Some(("SeqOpcodeTable", 0x8ABC)));
    assert_ne!(doctored, expected, "the byte gate is vacuous if a doctored carrier still matches");
}

/// t24 positive control (CONST `-D` axis): a doctored `SND_STAT_TICK` must make the
/// blob's abs-mem operands DIVERGE from the reference.
#[test]
fn blob_diverges_when_const_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let base = blob_lma(false) as usize;
    let expected = &refrom[base..base + BLOB_LEN_PLAIN];
    let doctored = native_blob_doctored(&aeon_dir(), false, Some(("SND_STAT_TICK", 0x1DED)));
    assert_ne!(doctored, expected, "a moved SND_STAT_TICK must change the blob's abs-mem operands");
}

/// The exported-symbol contract is present: `sound_sequencer.emp` exports all 26
/// handler labels the banked `seq_opcode_tab.asm` references (a missing one panics
/// in `native_sound_blob`, so reaching here means the contract is complete), and
/// each lies in the sequencer window.
#[test]
fn handler_symbol_contract_complete() {
    let Some(_) = read_ref("s4.bin") else { return };
    let syms = native_sound_blob(&aeon_dir(), false).symbols;
    assert_eq!(
        syms.len(),
        seam1::HANDLER_SYMBOLS.len(),
        "every seq_opcode_tab handler must be exported by the blob"
    );
    for (name, addr) in &syms {
        assert!(
            (0x0565..0x0CD7).contains(addr),
            "handler {name} at {addr:#x} must lie in the sequencer window $0565..$0CD7"
        );
    }
}

// ===========================================================================
// §2.4 — the whole-ROM gate (Option A / BINCLUDE): the native link IS the build —
// sigil emits the blob, asl (here sigil's AS front-end) BINCLUDEs it. The canonical
// bar is the ASSEMBLED ROM (0..EndOfRom + the two convsym header fields), which
// `assert_rom_matches_convsym` already scopes to; the deb2 debug-symbol append past
// EndOfRom legitimately shrinks (the deleted twins' labels leave the table) and is
// out of scope — Option B, overseer-ruled 2026-07-29.
// ===========================================================================

/// The three whole-ROM tests all read/write the SAME generated-blob path (fixed by
/// `boot_data.asm`'s BINCLUDE), so they must not race under cargo's parallel runner.
static GEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Ensure the generated BINCLUDE inputs (blob .bin + syms.asm) exist in the aeon
/// tree at the path `boot_data.asm`'s gate arm BINCLUDEs. Deterministic; gitignored.
fn ensure_generated(aeon: &Path) {
    seam1::emit_sound_blob(aeon, &aeon.join("engine/sound/generated"))
        .unwrap_or_else(|e| panic!("emit_sound_blob: {e}"));
}

/// Assemble the WHOLE ROM through sigil's AS front-end with the seam-1 gate ON (the
/// `boot_data.asm` arm BINCLUDEs the generated blob + includes the syms) and emit it.
fn build_seam1_rom(debug: bool) -> Vec<u8> {
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    ensure_generated(&aeon);
    let module = assemble_mixed_z80sound_as_side(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (seam1 BINCLUDE): {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link (seam1 BINCLUDE): {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (seam1 BINCLUDE): {e}"))
}

/// (PLAIN) the native-link BINCLUDE build == the canonical assembled `s4.bin`
/// (0..EndOfRom + the two convsym header fields, per `assert_rom_matches_convsym`).
#[test]
fn mixed_seam1_rom_matches_reference_plain() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let rom = build_seam1_rom(false);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        sigil_harness::pins::ASSEMBLED_LEN,
        "seam1 BINCLUDE (plain) vs s4.bin (assembled-ROM bar)",
    );
}

/// (DEBUG) the native-link BINCLUDE build == the canonical assembled `s4.debug.bin`.
#[test]
fn mixed_seam1_rom_matches_reference_debug() {
    let Some(refrom) = read_ref("s4.debug.bin") else { return };
    let rom = build_seam1_rom(true);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        sigil_harness::pins::DEBUG_ASSEMBLED_LEN,
        "seam1 BINCLUDE (debug) vs s4.debug.bin (assembled-ROM bar)",
    );
}

/// t24 WHOLE-ROM positive control (bite within the assembled-ROM scope): a DOCTORED
/// blob binary (`SND_STAT_TICK` moved) BINCLUDE'd into the ROM must DIVERGE from
/// canonical in the blob region — the gate is not vacuous. Re-emits the correct blob
/// afterward so sibling tests see the honest inputs.
#[test]
fn mixed_seam1_rom_diverges_when_blob_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let gen = aeon.join("engine/sound/generated");
    // Emit the honest inputs first (so syms.asm + the debug blob exist), THEN
    // overwrite ONLY the plain blob with a doctored one (SND_STAT_TICK moved).
    ensure_generated(&aeon);
    let doctored = native_blob_doctored(&aeon, false, Some(("SND_STAT_TICK", 0x1DED)));
    std::fs::write(gen.join("z80_sound_blob.bin"), &doctored).unwrap();
    let module = assemble_mixed_z80sound_as_side(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap()).unwrap();
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"));
    // Restore the honest blob for sibling tests.
    ensure_generated(&aeon);
    let base = blob_lma(false) as usize;
    assert_ne!(
        &rom[base..base + BLOB_LEN_PLAIN],
        &refrom[base..base + BLOB_LEN_PLAIN],
        "the whole-ROM gate is vacuous if a doctored blob still matches canonical"
    );
}
