//! seam-2 stage-2c Option Y — THE WHOLE-ROM MT BINCLUDE GATE (the real-build path,
//! both shapes). The dual proof for the Moving-Trucks `.asm`-stream deletion
//! (row-5 mt arm): assemble the REAL `games/sonic4/main.asm` with `SIGIL_EMP_DAC` +
//! `SIGIL_EMP_MT` on and NO body stubs, so the DAC banks/head AND the whole
//! Moving-Trucks streaming bank BINCLUDE (mt_bank{,_debug}.bin @ $58607) and
//! `sound_api.asm`'s `movea.l #SongTable`/`#SongPatchTable` resolve against the
//! emitted `mt_syms{,_debug}.asm` equs — exactly as `build.sh` assembles once the
//! seven MT `.asm` files are deleted. The whole ROM must equal the assembled
//! reference (`assert_rom_matches_convsym`) BOTH shapes.
//!
//! The MT bank is SHAPE-DEPENDENT (the debug build adds DrumTest + HCZ2 songs), so
//! there are per-shape `.bin` + syms artifacts. The region proof lives in `mt_port`
//! (the `.emp` composition) + the DSM `mixed_dac_rom` tranches; THIS gate proves the
//! BINCLUDE path (the real build) is byte-identical to it.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_mt_rom
//! ```

use sigil_harness::{assemble_seam2_mt_rom_as_side, assert_rom_matches_convsym, pins, seam1, seam2};
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

static GEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Emit every generated BINCLUDE input the DAC + MT + blob arms read.
fn ensure_generated(aeon: &Path) {
    let gen = aeon.join("engine/sound/generated");
    seam1::emit_sound_blob(aeon, &gen).unwrap_or_else(|e| panic!("emit_sound_blob (blob): {e}"));
    seam2::emit_dac_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_dac_artifacts: {e}"));
    seam2::emit_mt_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_mt_artifacts: {e}"));
    // soundBankHead now BINCLUDEs the SFX win-tab + seq-opcode heads (unconditional,
    // post stage-2d/3), so the whole-ROM assemble needs them present too.
    seam2::emit_sfx_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_sfx_artifacts: {e}"));
    seam2::emit_seq_opcode_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_seq_opcode_artifacts: {e}"));
    seam2::emit_sound_tables_artifacts(aeon, &gen).unwrap_or_else(|e| panic!("emit_sound_tables_artifacts: {e}"));
}

fn build_seam2_mt_rom(debug: bool) -> Vec<u8> {
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    ensure_generated(&aeon);
    let module = assemble_seam2_mt_rom_as_side(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (seam2 MT BINCLUDE): {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link (seam2 MT BINCLUDE): {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap())
        .unwrap_or_else(|e| panic!("load map: {e}"));
    sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (seam2 MT BINCLUDE): {e}"))
}

/// (PLAIN) the DAC+MT-BINCLUDE build == the canonical assembled `s4.bin`.
#[test]
fn seam2_mt_rom_matches_reference_plain() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let rom = build_seam2_mt_rom(false);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        pins::ASSEMBLED_LEN,
        "seam2 MT BINCLUDE (plain) vs s4.bin (assembled-ROM bar)",
    );
}

/// (DEBUG) the DAC+MT-BINCLUDE build == the canonical assembled `s4.debug.bin`.
#[test]
fn seam2_mt_rom_matches_reference_debug() {
    let Some(refrom) = read_ref("s4.debug.bin") else { return };
    let rom = build_seam2_mt_rom(true);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        pins::DEBUG_ASSEMBLED_LEN,
        "seam2 MT BINCLUDE (debug) vs s4.debug.bin (assembled-ROM bar)",
    );
}

/// t24 WHOLE-ROM positive control: a DOCTORED MT bank (one byte flipped) must make
/// the ROM DIVERGE from canonical in the MT window ($58607) — the gate is not
/// vacuous. Restores the honest inputs afterward.
#[test]
fn seam2_mt_rom_diverges_when_bank_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let aeon = aeon_dir();
    let gen = aeon.join("engine/sound/generated");
    ensure_generated(&aeon);
    let bank_path = gen.join("mt_bank.bin");
    let mut bank = std::fs::read(&bank_path).expect("read honest mt_bank");
    bank[0] ^= 0xFF; // flip the first song byte
    std::fs::write(&bank_path, &bank).expect("write doctored mt_bank");

    let module = assemble_seam2_mt_rom_as_side(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let resolved = sigil_link::resolve_layout(&module.sections, &Default::default(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &Default::default())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap()).unwrap();
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"));

    ensure_generated(&aeon); // restore honest inputs

    let lo = seam2::MT_BANK_LMA as usize;
    assert_ne!(
        &rom[lo..lo + 0x10],
        &refrom[lo..lo + 0x10],
        "the whole-ROM MT gate is vacuous if a doctored bank still matches canonical"
    );
}
